#!/usr/bin/env python3
"""
Convert filtered unicode ranges to Rust match patterns using u32 code points
"""


def convert_to_match_u32(input_file: str = "filtered_rust_ranges.txt",
                         output_file: str = "rust_match_u32.txt"):
    """
    Convert unicode ranges to Rust match patterns using u32 comparisons.
    Format: matches!(code, 0x0020..=0x007E | ...)
    """

    patterns = []

    with open(input_file, 'r', encoding='utf-8') as f:
        for line in f:
            line = line.strip()
            if not line or line.startswith('//'):
                continue

            # Convert hex format to u32 match pattern format
            if '..=' in line:
                # Range: 0x000020..=0x00007E -> 0x000020..=0x00007E
                patterns.append(line)
            else:
                # Single character: 0x00029E -> 0x00029E
                patterns.append(line)

    # Write as match arms
    with open(output_file, 'w', encoding='utf-8') as f:
        f.write("#[inline]\n")
        f.write("pub const fn is_safe_chars(c: char) -> bool {\n")
        f.write("    let code = c as u32;\n")
        f.write("    matches!(code,\n")

        # Write all patterns
        for i, pattern in enumerate(patterns):
            if i == len(patterns) - 1:
                # Last pattern - no pipe
                f.write(f"        {pattern}\n")
            else:
                f.write(f"        {pattern} |\n")

        f.write("    )\n")
        f.write("}\n")

    print(f"✓ Conversion complete!")
    print(f"✓ Generated {len(patterns)} match patterns")
    print(f"✓ Saved to: {output_file}")

    # Show preview
    print(f"\nPreview (first 15 patterns):")
    print("#[inline]")
    print("pub const fn is_safe_chars(c: char) -> bool {")
    print("    let code = c as u32;")
    print("    matches!(code,")
    for pattern in patterns[:15]:
        print(f"        {pattern} |")
    print(f"        ... and {len(patterns) - 15} more patterns")
    print("    )")
    print("}")

    # File size info
    import os
    file_size = os.path.getsize(output_file)
    print(f"\nFile size: {file_size:,} bytes ({file_size / 1024:.1f} KB)")


if __name__ == "__main__":
    convert_to_match_u32()
