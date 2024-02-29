import os
from PIL import Image

IMAGE_DIR = os.path.dirname(os.path.realpath(__file__))

all_files = os.listdir(IMAGE_DIR)
PNG_FILES = [f"{IMAGE_DIR}/{file}" for file in all_files if file.endswith(".png")]


def to_rgb16(rgba: tuple[int, int, int, int]) -> tuple[int, int]:
    r, g, b, a = rgba
    if a == 0:
        return (0xFF, 0xFF - 1)  # IGNORE

    g = (0b11111 * g // 255) << 11
    r = (0b111111 * r // 255) << 5
    b = 0b11111 * b // 255

    rgb = r | g | b

    return ((rgb >> 8) % 255, rgb & 0xFF)


for f in PNG_FILES:
    with Image.open(f) as img:
        img = img.convert("RGBA")

        rgba_vals = []
        for y in range(img.height):
            for x in range(img.width):
                rgba: tuple[int, int, int, int] = img.getpixel((x, y))
                rgba_vals.append(rgba)

        rgb16_vals = [to_rgb16(val) for val in rgba_vals]

        rgb565_filename = f[:-4] + ".rgb565"
        with open(rgb565_filename, "wb") as f:
            for msb, lsb in rgb16_vals:
                f.write(msb.to_bytes(byteorder="little", length=1))
                f.write(lsb.to_bytes(byteorder="little", length=1))
            print(f"generated {rgb565_filename}")
