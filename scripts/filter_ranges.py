#!/usr/bin/env python3
"""
Filter unicode ranges to keep only commonly used characters
Excludes control characters and rarely used symbols
"""

import unicodedata


def should_keep_character(char_code: int) -> bool:
    """
    Determine if a character should be kept.

    Keep:
    - Letters (L*): Lu, Ll, Lt, Lm, Lo
    - Numbers (N*): Nd, Nl, No
    - Common punctuation (P*): Pc, Pd, Ps, Pe, Pi, Pf, Po
    - Common symbols: Sc (currency), Some Sm (math), Some So (other symbols)
    - Spacing (Zs): Space separators
    - Marks (M*): Mn, Mc, Me (combining marks)

    Exclude:
    - Control characters (Cc)
    - Format characters (Cf)
    - Private use (Co)
    - Surrogates (Cs)
    - Unassigned (Cn)
    - Line/Paragraph separators (Zl, Zp)
    - Some modifier symbols (Sk) - can be selective
    """

    try:
        char = chr(char_code)
        category = unicodedata.category(char)

        # Exclude these categories
        exclude_categories = {
            'Cc',  # Control characters
            'Cf',  # Format characters
            'Co',  # Private use
            'Cs',  # Surrogate
            'Cn',  # Not assigned
            'Zl',  # Line separator
            'Zp',  # Paragraph separator
        }

        if category in exclude_categories:
            return False

        # Keep all letters
        if category.startswith('L'):
            return True

        # Keep all numbers
        if category.startswith('N'):
            return True

        # Keep all punctuation
        if category.startswith('P'):
            return True

        # Keep spacing
        if category == 'Zs':
            return True

        # Keep marks (combining characters)
        if category.startswith('M'):
            return True

        # Keep currency symbols
        if category == 'Sc':
            return True

        # Keep math symbols
        if category == 'Sm':
            return True

        # Keep modifier symbols (like ^, `, etc.)
        if category == 'Sk':
            return True

        # Keep other symbols (like ©, ®, °, etc.)
        if category == 'So':
            return True

        return False

    except Exception as e:
        return False


def filter_ranges(input_file: str = "parsed_rust_ranges.txt",
                  output_file: str = "filtered_rust_ranges.txt"):
    """
    Filter the parsed ranges to keep only commonly used characters.
    """

    kept_ranges = []
    excluded_count = 0
    kept_count = 0

    with open(input_file, 'r', encoding='utf-8') as f:
        for line in f:
            line = line.strip()
            if not line or line.startswith('//'):
                continue

            if '..=' in line:
                # Range
                start, end = line.split('..=')
                start_val = int(start, 16)
                end_val = int(end, 16)

                # Find contiguous ranges to keep
                current_range_start = None

                for val in range(start_val, end_val + 1):
                    if should_keep_character(val):
                        kept_count += 1
                        if current_range_start is None:
                            current_range_start = val
                    else:
                        excluded_count += 1
                        if current_range_start is not None:
                            # End of a kept range
                            if current_range_start == val - 1:
                                kept_ranges.append(f"0x{current_range_start:06X}")
                            else:
                                kept_ranges.append(f"0x{current_range_start:06X}..=0x{val-1:06X}")
                            current_range_start = None

                # Handle last range
                if current_range_start is not None:
                    if current_range_start == end_val:
                        kept_ranges.append(f"0x{current_range_start:06X}")
                    else:
                        kept_ranges.append(f"0x{current_range_start:06X}..=0x{end_val:06X}")

            else:
                # Single character
                val = int(line, 16)
                if should_keep_character(val):
                    kept_ranges.append(line)
                    kept_count += 1
                else:
                    excluded_count += 1

    # Write filtered ranges
    with open(output_file, 'w', encoding='utf-8') as f:
        f.write("// Filtered Rust Unicode Ranges (commonly used characters only)\n")
        f.write("// Excluded: Control characters, Format characters, Private use\n")
        f.write(f"// Total kept: {kept_count} characters\n")
        f.write(f"// Total excluded: {excluded_count} characters\n")
        f.write(f"// Range entries: {len(kept_ranges)}\n\n")

        for range_line in kept_ranges:
            f.write(range_line + '\n')

    print(f"✓ Filtering complete!")
    print(f"✓ Kept: {kept_count} characters")
    print(f"✓ Excluded: {excluded_count} characters")
    print(f"✓ Output ranges: {len(kept_ranges)}")
    print(f"✓ Saved to: {output_file}")

    # Show some examples of what was excluded
    print(f"\nExamples of excluded characters:")
    test_excludes = [0x00, 0x01, 0x1F, 0xAD, 0xE09F]
    for code in test_excludes:
        try:
            char = chr(code)
            cat = unicodedata.category(char)
            name = unicodedata.name(char, 'NO NAME')
            print(f"  U+{code:04X} [{cat}] {repr(char)} - {name}")
        except:
            pass

    # Show some examples of what was kept
    print(f"\nExamples of kept characters:")
    test_keeps = [0x41, 0x61, 0x30, 0x21, 0x24, 0x3042]
    for code in test_keeps:
        try:
            char = chr(code)
            cat = unicodedata.category(char)
            name = unicodedata.name(char, 'NO NAME')
            print(f"  U+{code:04X} [{cat}] {repr(char)} - {name}")
        except:
            pass


if __name__ == "__main__":
    filter_ranges()
