#!/usr/bin/env python3
"""
Analyze the unicode ranges to categorize characters
"""

import unicodedata
from collections import defaultdict


def analyze_ranges(input_file: str = "parsed_rust_ranges.txt"):
    """Analyze and categorize all characters in the ranges."""

    category_stats = defaultdict(list)

    with open(input_file, 'r', encoding='utf-8') as f:
        for line in f:
            line = line.strip()
            if not line or line.startswith('//'):
                continue

            if '..=' in line:
                start, end = line.split('..=')
                start_val = int(start, 16)
                end_val = int(end, 16)

                for val in range(start_val, end_val + 1):
                    try:
                        char = chr(val)
                        cat = unicodedata.category(char)
                        category_stats[cat].append((val, char))
                    except:
                        pass
            else:
                val = int(line, 16)
                try:
                    char = chr(val)
                    cat = unicodedata.category(char)
                    category_stats[cat].append((val, char))
                except:
                    pass

    # Print statistics
    print("Unicode Category Statistics:")
    print("=" * 80)

    category_names = {
        'Cc': 'Control characters',
        'Cf': 'Format characters',
        'Cs': 'Surrogate',
        'Co': 'Private use',
        'Cn': 'Not assigned',
        'Lu': 'Uppercase letters',
        'Ll': 'Lowercase letters',
        'Lt': 'Titlecase letters',
        'Lm': 'Modifier letters',
        'Lo': 'Other letters',
        'Mn': 'Non-spacing marks',
        'Mc': 'Spacing combining marks',
        'Me': 'Enclosing marks',
        'Nd': 'Decimal numbers',
        'Nl': 'Letter numbers',
        'No': 'Other numbers',
        'Pc': 'Connector punctuation',
        'Pd': 'Dash punctuation',
        'Ps': 'Open punctuation',
        'Pe': 'Close punctuation',
        'Pi': 'Initial quote punctuation',
        'Pf': 'Final quote punctuation',
        'Po': 'Other punctuation',
        'Sm': 'Math symbols',
        'Sc': 'Currency symbols',
        'Sk': 'Modifier symbols',
        'So': 'Other symbols',
        'Zs': 'Space separators',
        'Zl': 'Line separators',
        'Zp': 'Paragraph separators',
    }

    for cat in sorted(category_stats.keys()):
        count = len(category_stats[cat])
        name = category_names.get(cat, 'Unknown')
        print(f"\n{cat} - {name}: {count} characters")

        # Show some examples
        examples = category_stats[cat][:10]
        for val, char in examples:
            try:
                char_name = unicodedata.name(char, 'NO NAME')
                print(f"  U+{val:04X} {repr(char):6s} - {char_name}")
            except:
                print(f"  U+{val:04X} {repr(char):6s}")

        if len(category_stats[cat]) > 10:
            print(f"  ... and {len(category_stats[cat]) - 10} more")


if __name__ == "__main__":
    analyze_ranges()
