#!/usr/bin/env python3
"""
eztrans_translation_results.csv에서 matches가 TRUE인 유니코드 범위를 추출하는 스크립트

이 스크립트는 번역 결과가 일치하는 문자들의 유니코드 범위를 찾아서
연속된 범위로 그룹화하여 텍스트 파일로 저장합니다.
"""

import csv
from pathlib import Path
from typing import List, Tuple


def parse_unicode_code(code: str) -> int:
    """유니코드 코드 포인트를 파싱 (예: U+000020 -> 32)"""
    if code.startswith('U+'):
        return int(code[2:], 16)
    return int(code, 16)


def format_unicode_code(code_point: int) -> str:
    """코드 포인트를 U+XXXX 형식으로 포맷"""
    return f"U+{code_point:06X}"


def find_unicode_ranges(code_points: List[int]) -> List[Tuple[int, int]]:
    """연속된 유니코드 코드 포인트들을 범위로 그룹화"""
    if not code_points:
        return []

    # 정렬
    sorted_points = sorted(set(code_points))

    ranges = []
    start = sorted_points[0]
    end = sorted_points[0]

    for point in sorted_points[1:]:
        if point == end + 1:
            # 연속된 범위 확장
            end = point
        else:
            # 새로운 범위 시작
            ranges.append((start, end))
            start = point
            end = point

    # 마지막 범위 추가
    ranges.append((start, end))

    return ranges


def get_unicode_block_name(code_point: int) -> str:
    """유니코드 블록 이름 추정"""
    # 주요 유니코드 블록들
    blocks = [
        (0x0000, 0x007F, "Basic Latin"),
        (0x0080, 0x00FF, "Latin-1 Supplement"),
        (0x0100, 0x017F, "Latin Extended-A"),
        (0x0180, 0x024F, "Latin Extended-B"),
        (0x0250, 0x02AF, "IPA Extensions"),
        (0x02B0, 0x02FF, "Spacing Modifier Letters"),
        (0x0300, 0x036F, "Combining Diacritical Marks"),
        (0x0370, 0x03FF, "Greek and Coptic"),
        (0x0400, 0x04FF, "Cyrillic"),
        (0x0500, 0x052F, "Cyrillic Supplement"),
        (0x1E00, 0x1EFF, "Latin Extended Additional"),
        (0x2000, 0x206F, "General Punctuation"),
        (0x2070, 0x209F, "Superscripts and Subscripts"),
        (0x20A0, 0x20CF, "Currency Symbols"),
        (0x2100, 0x214F, "Letterlike Symbols"),
        (0x2150, 0x218F, "Number Forms"),
        (0x2190, 0x21FF, "Arrows"),
        (0x2200, 0x22FF, "Mathematical Operators"),
        (0x2300, 0x23FF, "Miscellaneous Technical"),
        (0x2400, 0x243F, "Control Pictures"),
        (0x2460, 0x24FF, "Enclosed Alphanumerics"),
        (0x2500, 0x257F, "Box Drawing"),
        (0x2580, 0x259F, "Block Elements"),
        (0x25A0, 0x25FF, "Geometric Shapes"),
        (0x2600, 0x26FF, "Miscellaneous Symbols"),
        (0x3000, 0x303F, "CJK Symbols and Punctuation"),
        (0x3040, 0x309F, "Hiragana"),
        (0x30A0, 0x30FF, "Katakana"),
        (0x3100, 0x312F, "Bopomofo"),
        (0x3130, 0x318F, "Hangul Compatibility Jamo"),
        (0x3190, 0x319F, "Kanbun"),
        (0x31A0, 0x31BF, "Bopomofo Extended"),
        (0x3200, 0x32FF, "Enclosed CJK Letters and Months"),
        (0x3300, 0x33FF, "CJK Compatibility"),
        (0x4E00, 0x9FFF, "CJK Unified Ideographs"),
        (0xAC00, 0xD7AF, "Hangul Syllables"),
        (0xF900, 0xFAFF, "CJK Compatibility Ideographs"),
        (0xFE30, 0xFE4F, "CJK Compatibility Forms"),
        (0xFF00, 0xFFEF, "Halfwidth and Fullwidth Forms"),
    ]

    for start, end, name in blocks:
        if start <= code_point <= end:
            return name

    return "Unknown"


def extract_matched_unicode_ranges(csv_file_path: Path) -> List[int]:
    """matches가 TRUE인 유니코드 코드 포인트들을 추출"""
    matched_codes = []

    try:
        with open(csv_file_path, 'r', encoding='utf-8-sig') as csvfile:
            reader = csv.DictReader(csvfile)

            for row in reader:
                # matches 칼럼 확인 (대소문자 무시)
                matches = row.get('matches', '').strip().lower()

                if matches in ['true', '1', 'yes']:
                    code = row.get('code', '').strip()
                    if code:
                        try:
                            code_point = parse_unicode_code(code)
                            matched_codes.append(code_point)
                        except ValueError as e:
                            print(f"경고: 코드 파싱 실패 - {code}: {e}")

    except FileNotFoundError:
        print(f"오류: {csv_file_path} 파일을 찾을 수 없습니다.")
        return []

    return matched_codes


def save_ranges_to_text(ranges: List[Tuple[int, int]], output_file: Path, matched_codes: List[int]):
    """유니코드 범위를 텍스트 파일로 저장"""
    with open(output_file, 'w', encoding='utf-8') as f:
        f.write("=" * 80 + "\n")
        f.write("EzTrans 번역 결과가 일치하는 유니코드 범위\n")
        f.write("=" * 80 + "\n\n")

        f.write(f"총 일치하는 문자 수: {len(matched_codes)}\n")
        f.write(f"총 범위 수: {len(ranges)}\n\n")

        # 범위별 상세 정보
        f.write("=" * 80 + "\n")
        f.write("범위 목록\n")
        f.write("=" * 80 + "\n\n")

        for i, (start, end) in enumerate(ranges, 1):
            count = end - start + 1
            start_block = get_unicode_block_name(start)
            end_block = get_unicode_block_name(end)

            f.write(f"{i}. {format_unicode_code(start)}")

            if start != end:
                f.write(f" - {format_unicode_code(end)}")
                f.write(f" ({count}개 문자)")

            f.write("\n")

            # 블록 정보
            if start_block == end_block:
                f.write(f"   블록: {start_block}\n")
            else:
                f.write(f"   블록: {start_block} ~ {end_block}\n")

            # 샘플 문자 표시 (처음 10개만)
            sample_chars = []
            for cp in range(start, min(start + 10, end + 1)):
                try:
                    sample_chars.append(chr(cp))
                except ValueError:
                    sample_chars.append('?')

            if sample_chars:
                f.write(f"   샘플: {' '.join(sample_chars)}")
                if end - start >= 10:
                    f.write(" ...")
                f.write("\n")

            f.write("\n")

        # Rust 코드 형식으로도 출력
        f.write("\n" + "=" * 80 + "\n")
        f.write("Rust 코드 형식 (배열)\n")
        f.write("=" * 80 + "\n\n")

        f.write("const MATCHED_UNICODE_RANGES: &[(u32, u32)] = &[\n")
        for start, end in ranges:
            f.write(f"    (0x{start:06X}, 0x{end:06X}),  // {format_unicode_code(start)}")
            if start != end:
                f.write(f" - {format_unicode_code(end)}")
            f.write(f" ({get_unicode_block_name(start)})\n")
        f.write("];\n\n")

        # Python 코드 형식으로도 출력
        f.write("\n" + "=" * 80 + "\n")
        f.write("Python 코드 형식 (리스트)\n")
        f.write("=" * 80 + "\n\n")

        f.write("MATCHED_UNICODE_RANGES = [\n")
        for start, end in ranges:
            f.write(f"    (0x{start:06X}, 0x{end:06X}),  # {format_unicode_code(start)}")
            if start != end:
                f.write(f" - {format_unicode_code(end)}")
            f.write(f" ({get_unicode_block_name(start)})\n")
        f.write("]\n\n")

        # 통계
        f.write("\n" + "=" * 80 + "\n")
        f.write("블록별 통계\n")
        f.write("=" * 80 + "\n\n")

        # 블록별로 그룹화
        block_stats = {}
        for cp in matched_codes:
            block = get_unicode_block_name(cp)
            block_stats[block] = block_stats.get(block, 0) + 1

        for block, count in sorted(block_stats.items(), key=lambda x: -x[1]):
            f.write(f"{block}: {count}개\n")


def main():
    # 입력/출력 파일 경로
    csv_file = Path(__file__).parent / 'eztrans_translation_results.csv'
    output_file = Path(__file__).parent / 'matched_unicode_ranges.txt'

    if not csv_file.exists():
        print(f"오류: {csv_file} 파일을 찾을 수 없습니다.")
        print("먼저 translate_csv 예제를 실행하여 번역 결과 CSV를 생성하세요.")
        return

    print("CSV 파일 읽는 중...")
    matched_codes = extract_matched_unicode_ranges(csv_file)

    if not matched_codes:
        print("일치하는 유니코드 문자를 찾을 수 없습니다.")
        return

    print(f"일치하는 문자 {len(matched_codes)}개 발견")

    # 범위 계산
    print("유니코드 범위 계산 중...")
    ranges = find_unicode_ranges(matched_codes)

    print(f"총 {len(ranges)}개의 범위로 그룹화됨")

    # 파일로 저장
    print(f"\n결과 저장 중: {output_file}")
    save_ranges_to_text(ranges, output_file, matched_codes)

    print("저장 완료!")

    # 요약 출력
    print("\n" + "=" * 80)
    print("요약")
    print("=" * 80)
    print(f"총 일치하는 문자: {len(matched_codes)}개")
    print(f"총 범위: {len(ranges)}개")
    print(f"\n주요 범위 (상위 5개):")

    # 범위 크기별 정렬
    sorted_ranges = sorted(ranges, key=lambda r: r[1] - r[0] + 1, reverse=True)
    for i, (start, end) in enumerate(sorted_ranges[:5], 1):
        count = end - start + 1
        block = get_unicode_block_name(start)
        print(f"{i}. {format_unicode_code(start)} - {format_unicode_code(end)}: {count}개 ({block})")


if __name__ == '__main__':
    main()
