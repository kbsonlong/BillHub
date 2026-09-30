#!/usr/bin/env python3
"""Generate BillHub app icon PNG from vector description."""

from PIL import Image, ImageDraw


def quadratic_bezier(p0, p1, p2, steps=40):
    """Approximate a quadratic bezier curve with line segments."""
    points = []
    for i in range(steps + 1):
        t = i / steps
        x = (1 - t) ** 2 * p0[0] + 2 * (1 - t) * t * p1[0] + t ** 2 * p2[0]
        y = (1 - t) ** 2 * p0[1] + 2 * (1 - t) * t * p1[1] + t ** 2 * p2[1]
        points.append((x, y))
    return points


def scale_point(p, factor):
    return (p[0] * factor, p[1] * factor)


def build_b_outline(scale=2.0):
    """Build the B letterform outline in target coordinates."""
    # Coordinates are in a 512x512 conceptual space.
    # Start top-left of stem, go clockwise around the B shape.
    outline = []

    # 1. Top edge of stem to top bump start
    outline.append((160, 124))
    outline.append((290, 124))

    # 2. Top bump outer curve: (290,124) -> (346,124) -> (346,172)
    outline.extend(quadratic_bezier((290, 124), (346, 124), (346, 172)))
    # Top bump inner curve: (346,172) -> (346,220) -> (290,220)
    outline.extend(quadratic_bezier((346, 172), (346, 220), (290, 220)))

    # 3. Bottom bump outer curve: (290,220) -> (370,220) -> (370,296)
    outline.extend(quadratic_bezier((290, 220), (370, 220), (370, 296)))
    # Bottom bump inner curve: (370,296) -> (370,376) -> (290,376)
    outline.extend(quadratic_bezier((370, 296), (370, 376), (290, 376)))

    # 4. Bottom edge back to stem left
    outline.append((160, 376))

    # Scale to target size
    return [scale_point(p, scale) for p in outline]


def generate_icon(size=1024, output_path="icon.png"):
    """Generate the app icon at the given size."""
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    # Background rounded square (iOS/macOS app icon shape)
    bg_color = (34, 107, 79, 255)  # #226b4f
    corner_radius = int(size * 112 / 512)
    draw.rounded_rectangle((0, 0, size, size), radius=corner_radius, fill=bg_color)

    # B letterform in cream white
    b_color = (255, 253, 248, 255)  # #fffdf8
    scale = size / 512.0
    outline = build_b_outline(scale)
    draw.polygon(outline, fill=b_color)

    img.save(output_path, "PNG")
    print(f"Saved icon: {output_path} ({size}x{size})")


if __name__ == "__main__":
    import os

    base_dir = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    icons_dir = os.path.join(base_dir, "src-tauri", "icons")
    os.makedirs(icons_dir, exist_ok=True)

    generate_icon(1024, os.path.join(icons_dir, "icon.png"))
    generate_icon(512, os.path.join(icons_dir, "icon-512.png"))
