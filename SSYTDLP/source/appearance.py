from pathlib import Path
from PIL import Image, ImageOps

PALETTES = {
    'light': dict(bg='#eaf0f8', panel='#ffffff', text='#18283e', muted='#61738b', field='#f5f8fc', border='#d7e1ee', accent='#2569c9', button='#e8eef7', active='#d6e4f7', selected='#d6e6ff'),
    'dark': dict(bg='#101720', panel='#1a2432', text='#edf3ff', muted='#a0b0c7', field='#111b29', border='#34445b', accent='#4289ef', button='#2a3a50', active='#3b5270', selected='#284c7b'),
}


def read_wallpaper(path: str) -> Image.Image | None:
    if not path:
        return None
    with Image.open(path) as image:
        if image.width * image.height > 40_000_000:
            raise ValueError('Image is too large.')
        result = ImageOps.exif_transpose(image).convert('RGB')
        result.thumbnail((3840, 2160), Image.Resampling.LANCZOS)
        return result


def import_wallpaper(source: str, destination: Path) -> str:
    image = read_wallpaper(source)
    if image is None:
        return ''
    destination.parent.mkdir(parents=True, exist_ok=True)
    temporary = destination.with_suffix('.tmp')
    image.save(temporary, format='PNG')
    temporary.replace(destination)
    return str(destination)
