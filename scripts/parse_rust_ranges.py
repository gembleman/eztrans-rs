#!/usr/bin/env python3
"""
Parse Rust unicode ranges from full_unicode_scan_v3_results.txt
Extracts only the 'Generated Rust ranges (safe chars):' section
"""

import re
from pathlib import Path


def parse_rust_ranges(input_file: str, output_file: str = "parsed_rust_ranges.txt"):
    """
    Parse Rust ranges from the input file and save to output file.

    Args:
        input_file: Path to full_unicode_scan_v3_results.txt
        output_file: Path to save parsed ranges
    """

    ranges = []
    in_rust_section = False

    # Read the input file
    try:
        with open(input_file, 'r', encoding='utf-8') as f:
            for line in f:
                line = line.strip()

                # Check if we've reached the Rust ranges section
                if 'Generated Rust ranges (safe chars):' in line:
                    in_rust_section = True
                    continue

                # Stop when we reach the end of the section (empty line or next section)
                if in_rust_section:
                    # Empty line or line starting with text (not hex) indicates end of section
                    if not line or (line and not line.startswith('0x')):
                        break

                    # Parse the range line
                    if line.startswith('0x'):
                        ranges.append(line)

    except FileNotFoundError:
        print(f"Error: File '{input_file}' not found.")
        return
    except Exception as e:
        print(f"Error reading file: {e}")
        return

    # Save the parsed ranges
    try:
        with open(output_file, 'w', encoding='utf-8') as f:
            f.write("// Parsed Rust Unicode Ranges (safe chars)\n")
            f.write(f"// Total ranges: {len(ranges)}\n")
            f.write("// Format: 0xHEXVALUE or 0xHEXVALUE..=0xHEXVALUE\n\n")

            for range_line in ranges:
                f.write(range_line + '\n')

        print(f"✓ Successfully parsed {len(ranges)} ranges")
        print(f"✓ Saved to: {output_file}")

        # Display some statistics
        single_chars = sum(1 for r in ranges if '..=' not in r)
        range_chars = len(ranges) - single_chars
        print(f"\nStatistics:")
        print(f"  - Single characters: {single_chars}")
        print(f"  - Range entries: {range_chars}")

        # Show first and last few entries
        print(f"\nFirst 5 entries:")
        for r in ranges[:5]:
            print(f"  {r}")

        print(f"\nLast 5 entries:")
        for r in ranges[-5:]:
            print(f"  {r}")

    except Exception as e:
        print(f"Error writing output file: {e}")


def main():
    # Default paths
    input_file = r"C:\Users\nsoop\Documents\project\rust_project\eztrans_project\eztrans-rs\full_unicode_scan_v3_results.txt"
    output_file = "parsed_rust_ranges.txt"

    # Check if input file exists
    if not Path(input_file).exists():
        print(f"Input file not found: {input_file}")
        print("Please provide the correct path to full_unicode_scan_v3_results.txt")
        return

    print(f"Parsing Rust ranges from: {input_file}")
    parse_rust_ranges(input_file, output_file)


if __name__ == "__main__":
    main()
