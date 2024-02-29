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


def as_byte(n: int) -> bytes:
    return n.to_bytes(byteorder="little", length=1)


def byte_from_bits(bits: list[bool]) -> bytes:
    n = 0
    for bit in bits:
        n |= 1 if bit else 0
        n <<= 1
    n >>= 1
    return as_byte(n)


for f in PNG_FILES:
    with Image.open(f) as img:
        img = img.convert("RGBA")

        rgba_vals: list[tuple[int, int, int, int]] = []
        for y in range(img.height):
            for x in range(img.width):
                rgba: tuple[int, int, int, int] = img.getpixel((x, y))
                rgba_vals.append(rgba)

        rgb16_vals = [to_rgb16(val) for val in rgba_vals]

        rgb565_filename = f[:-4] + ".rgb565"
        with open(rgb565_filename, "wb") as f2:
            for msb, lsb in rgb16_vals:
                f2.write(as_byte(msb))
                f2.write(as_byte(lsb))
            print(f"+ {rgb565_filename}")

        # now for the bitmask image

        bmi_bits = [val[3] != 0 for val in rgba_vals]
        for i in range(len(bmi_bits) % 8):
            bmi_bits.append(False)

        bmi_bytes: list[bytes] = []
        for i in range(len(bmi_bits) // 8):
            bmi_bytes.append(byte_from_bits(bmi_bits[i * 8 : i * 8 + 8]))

        bmi_filename = f[:-4] + ".bmi"
        with open(bmi_filename, "wb") as f2:
            for byte in bmi_bytes:
                f2.write(byte)
            print(f"+ {bmi_filename}")
