use crate::model::*;
use anyhow::{Context, Result, bail};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::SaltString};
use rusqlite::{Connection as Sqlite, OptionalExtension, params};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub struct Store {
    pub root: PathBuf,
    pub db: Sqlite,
}
impl Store {
    pub fn open(root: &Path) -> Result<Self> {
        std::fs::create_dir_all(root)?;
        for dir in [
            "imports",
            "imports/processed",
            "imports/failed",
            "artwork",
            "offline",
        ] {
            std::fs::create_dir_all(root.join(dir))?;
        }
        let db = Sqlite::open(root.join("library.sqlite3"))?;
        db.busy_timeout(std::time::Duration::from_secs(8))?;
        db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;
            CREATE TABLE IF NOT EXISTS media(id TEXT PRIMARY KEY,path TEXT NOT NULL UNIQUE,data TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS states(user_id TEXT PRIMARY KEY,data TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS users(id TEXT PRIMARY KEY,name TEXT NOT NULL UNIQUE,password TEXT NOT NULL,admin INTEGER NOT NULL DEFAULT 0);
            CREATE TABLE IF NOT EXISTS sessions(hash TEXT PRIMARY KEY,user_id TEXT NOT NULL REFERENCES users(id),created INTEGER NOT NULL);
            CREATE TABLE IF NOT EXISTS meta(key TEXT PRIMARY KEY,value TEXT NOT NULL);")?;
        Ok(Self {
            root: root.into(),
            db,
        })
    }
    pub fn media(&self) -> Result<Vec<Media>> {
        let mut stmt=self.db.prepare("SELECT data FROM media ORDER BY json_extract(data,'$.artist'),json_extract(data,'$.album'),json_extract(data,'$.track'),json_extract(data,'$.title')")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        rows.map(|r| Ok(serde_json::from_str(&r?)?)).collect()
    }
    pub fn get(&self, id: &str) -> Result<Option<Media>> {
        let s = self
            .db
            .query_row("SELECT data FROM media WHERE id=?1", [id], |r| {
                r.get::<_, String>(0)
            })
            .optional()?;
        s.map(|s| Ok(serde_json::from_str(&s)?)).transpose()
    }
    pub fn by_path(&self, path: &str) -> Result<Option<Media>> {
        let s = self
            .db
            .query_row("SELECT data FROM media WHERE path=?1", [path], |r| {
                r.get::<_, String>(0)
            })
            .optional()?;
        s.map(|s| Ok(serde_json::from_str(&s)?)).transpose()
    }
    pub fn by_fingerprint(&self, fingerprint: &str) -> Result<Option<Media>> {
        let s = self
            .db
            .query_row(
                "SELECT data FROM media WHERE json_extract(data,'$.fingerprint')=?1 LIMIT 1",
                [fingerprint],
                |r| r.get::<_, String>(0),
            )
            .optional()?;
        s.map(|s| Ok(serde_json::from_str(&s)?)).transpose()
    }
    pub fn put(&self, m: &Media) -> Result<()> {
        // A rewritten file at an existing path may have a new byte identity; retain the user's existing identity.
        self.db.execute("INSERT INTO media(id,path,data) VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET path=excluded.path,data=excluded.data",params![m.id,m.path,serde_json::to_string(m)?])?;
        Ok(())
    }
    pub fn put_many(&self, rows: &[Media]) -> Result<()> {
        let tx = self.db.unchecked_transaction()?;
        {
            let mut statement=tx.prepare("INSERT INTO media(id,path,data) VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET path=excluded.path,data=excluded.data")?;
            for m in rows {
                statement.execute(params![m.id, m.path, serde_json::to_string(m)?])?;
            }
        }
        tx.commit()?;
        Ok(())
    }
    pub fn state(&self, user: &str) -> Result<UserState> {
        let s = self
            .db
            .query_row("SELECT data FROM states WHERE user_id=?1", [user], |r| {
                r.get::<_, String>(0)
            })
            .optional()?;
        match s {
            Some(s) => Ok(serde_json::from_str(&s)?),
            None => Ok(UserState::default()),
        }
    }
    pub fn save_state(&self, user: &str, state: &UserState) -> Result<()> {
        self.db.execute("INSERT INTO states VALUES(?1,?2) ON CONFLICT(user_id) DO UPDATE SET data=excluded.data",params![user,serde_json::to_string(state)?])?;
        Ok(())
    }
    pub fn compare_save(&mut self, user: &str, state: &mut UserState) -> Result<()> {
        let tx = self
            .db
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let previous: Option<String> = tx
            .query_row("SELECT data FROM states WHERE user_id=?1", [user], |r| {
                r.get(0)
            })
            .optional()?;
        let previous = previous
            .map(|s| serde_json::from_str::<UserState>(&s))
            .transpose()?
            .unwrap_or_default();
        let rev = previous.revision;
        if state.revision != rev {
            bail!("State changed on another device; refresh before saving")
        }
        // A 0.2 client has no period-detail fields. Preserve those when it
        // saves ordinary listening changes; an explicit history reset still
        // clears everything because it also resets stats_since to zero.
        if state.stats_detail_since == 0 && previous.stats_detail_since > 0 && state.stats_since > 0
        {
            state.months = previous.months;
            state.first_heard = previous.first_heard;
            state.stats_detail_since = previous.stats_detail_since;
            if state.lyrics_overrides.is_empty() {
                state.lyrics_overrides = previous.lyrics_overrides;
            }
        }
        state.revision += 1;
        tx.execute("INSERT INTO states VALUES(?1,?2) ON CONFLICT(user_id) DO UPDATE SET data=excluded.data",params![user,serde_json::to_string(state)?])?;
        tx.commit()?;
        Ok(())
    }
    pub fn add_user(&self, name: &str, password: &str, admin: bool) -> Result<()> {
        if name.trim().is_empty() || name.len() > 80 || password.len() < 10 || password.len() > 512
        {
            bail!("Use a username and a password of 10–512 characters")
        }
        let salt = SaltString::generate(&mut rand::rngs::OsRng);
        let hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| anyhow::anyhow!(e.to_string()))?
            .to_string();
        self.db
            .execute(
                "INSERT INTO users VALUES(?1,?2,?3,?4)",
                params![crate::token(), name, hash, admin],
            )
            .context("Username already exists")?;
        Ok(())
    }
    pub fn login(&self, l: &Login) -> Result<Session> {
        if l.username.len() > 80 || l.password.len() > 512 {
            bail!("Invalid credentials")
        }
        let row: Option<(String, String, bool)> = self
            .db
            .query_row(
                "SELECT id,password,admin FROM users WHERE name=?1",
                [&l.username],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        let (id, hash, admin) = row.context("Invalid credentials")?;
        Argon2::default()
            .verify_password(
                l.password.as_bytes(),
                &PasswordHash::new(&hash).map_err(|_| anyhow::anyhow!("Invalid credentials"))?,
            )
            .map_err(|_| anyhow::anyhow!("Invalid credentials"))?;
        let token = crate::token();
        let digest = hex::encode(Sha256::digest(token.as_bytes()));
        self.db.execute(
            "DELETE FROM sessions WHERE created<?1",
            [crate::now().saturating_sub(30 * 86400) as i64],
        )?;
        self.db.execute(
            "INSERT INTO sessions VALUES(?1,?2,?3)",
            params![digest, id, crate::now() as i64],
        )?;
        Ok(Session {
            token,
            username: l.username.clone(),
            admin,
        })
    }
    pub fn authenticate(&self, token: &str) -> Result<Option<(String, bool)>> {
        if token.len() != 64 {
            return Ok(None);
        }
        let hash = hex::encode(Sha256::digest(token.as_bytes()));
        Ok(self.db.query_row("SELECT u.id,u.admin FROM users u JOIN sessions s ON s.user_id=u.id WHERE s.hash=?1 AND s.created>?2",params![hash,crate::now().saturating_sub(30*86400) as i64],|r|Ok((r.get(0)?,r.get(1)?))).optional()?)
    }
    pub fn revoke(&self, token: &str) -> Result<()> {
        self.db.execute(
            "DELETE FROM sessions WHERE hash=?1",
            [hex::encode(Sha256::digest(token.as_bytes()))],
        )?;
        Ok(())
    }
    pub fn backup(&self, destination: &Path) -> Result<()> {
        // SQLite creates a consistent snapshot including committed WAL data.
        self.db
            .execute("VACUUM INTO ?1", [destination.to_string_lossy().as_ref()])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bulk_metadata_is_atomic_and_legacy_clients_keep_period_details() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(dir.path()).unwrap();
        let first = Media {
            id: "a".into(),
            path: "a.wav".into(),
            title: "Original".into(),
            ..Default::default()
        };
        store.put(&first).unwrap();
        let changed = Media {
            title: "Changed".into(),
            ..first.clone()
        };
        let conflicting = Media {
            id: "b".into(),
            path: "a.wav".into(),
            ..Default::default()
        };
        assert!(store.put_many(&[changed, conflicting]).is_err());
        assert_eq!(store.get("a").unwrap().unwrap().title, "Original");
        let mut state = UserState::default();
        state.preferences.history = true;
        state.record_seconds("a", 1., crate::stats::timestamp(2026, 10) + 1);
        store.save_state("profile", &state).unwrap();
        let mut old = state.clone();
        old.months.clear();
        old.first_heard.clear();
        old.stats_detail_since = 0;
        store.compare_save("profile", &mut old).unwrap();
        assert!(!old.months.is_empty());
        old.clear_history();
        store.compare_save("profile", &mut old).unwrap();
        assert!(old.months.is_empty());
    }
    #[test]
    fn durable_states_and_history_controls() {
        let d = tempfile::tempdir().unwrap();
        let s = Store::open(d.path()).unwrap();
        let mut a = UserState::default();
        a.listen("track");
        assert!(a.history.is_empty());
        a.preferences.history = true;
        a.listen("track");
        s.save_state("alice", &a).unwrap();
        assert_eq!(s.state("alice").unwrap().counts["track"], 1);
        assert!(s.state("bob").unwrap().history.is_empty());
    }
    #[test]
    fn credentials_sessions_and_revocation() {
        let d = tempfile::tempdir().unwrap();
        let s = Store::open(d.path()).unwrap();
        s.add_user("owner", "a-long-password", true).unwrap();
        assert!(
            s.login(&Login {
                username: "owner".into(),
                password: "wrong".into()
            })
            .is_err()
        );
        let t = s
            .login(&Login {
                username: "owner".into(),
                password: "a-long-password".into(),
            })
            .unwrap();
        assert!(s.authenticate(&t.token).unwrap().unwrap().1);
        s.revoke(&t.token).unwrap();
        assert!(s.authenticate(&t.token).unwrap().is_none());
    }
    #[test]
    fn concurrent_state_changes_are_rejected() {
        let d = tempfile::tempdir().unwrap();
        let mut s = Store::open(d.path()).unwrap();
        let mut first = s.state("u").unwrap();
        let mut stale = first.clone();
        s.compare_save("u", &mut first).unwrap();
        assert!(s.compare_save("u", &mut stale).is_err());
    }
}
