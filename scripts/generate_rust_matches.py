#!/usr/bin/env python3
"""
matched_unicode_ranges.txt에서 Rust matches! 매크로 형식으로 변환하는 스크립트
"""

import csv
from pathlib import Path
from typing import List, Tuple


def parse_unicode_code(code: str) -> int:
    """유니코드 코드 포인트를 파싱 (예: U+000020 -> 32)"""
    if code.startswith('U+'):
        return int(code[2:], 16)
    return int(code, 16)


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


def generate_rust_matches_macro(ranges: List[Tuple[int, int]], output_file: Path):
    """Rust matches! 매크로 형식으로 출력"""
    with open(output_file, 'w', encoding='utf-8') as f:
        # 함수 정의
        f.write("#[inline]\n")
        f.write("pub const fn is_safe_chars(c: char) -> bool {\n")
        f.write("    let code = c as u32;\n")
        f.write("    matches!(code,\n")

        # 범위들을 출력
        for i, (start, end) in enumerate(ranges):
            if start == end:
                # 단일 코드 포인트
                f.write(f"        0x{start:06X}")
            else:
                # 범위
                f.write(f"        0x{start:06X}..=0x{end:06X}")

            # 마지막이 아니면 | 추가
            if i < len(ranges) - 1:
                f.write(" |")

            f.write("\n")

        f.write("    )\n")
        f.write("}\n\n")

        # 테스트 코드 추가
        f.write("#[cfg(test)]\n")
        f.write("mod tests {\n")
        f.write("    use super::*;\n\n")
        f.write("    #[test]\n")
        f.write("    fn test_safe_chars() {\n")
        f.write("        // 안전한 문자 테스트\n")
        f.write("        assert!(is_safe_chars(' '));  // U+000020\n")
        f.write("        assert!(is_safe_chars('A'));  // U+000041\n")
        f.write("        assert!(is_safe_chars('À'));  // U+0000C0\n")
        f.write("        \n")
        f.write("        // 안전하지 않은 문자 테스트 (전각 문자)\n")
        f.write("        assert!(!is_safe_chars('Ａ')); // U+FF21 (전각 A)\n")
        f.write("        assert!(!is_safe_chars('０')); // U+FF10 (전각 0)\n")
        f.write("        assert!(!is_safe_chars('　')); // U+003000 (전각 공백)\n")
        f.write("    }\n")
        f.write("}\n")


def generate_compact_format(ranges: List[Tuple[int, int]], output_file: Path):
    """여러 형식으로 출력"""
    with open(output_file, 'w', encoding='utf-8') as f:
        f.write("=" * 80 + "\n")
        f.write("Rust matches! 매크로 형식 - is_safe_chars 함수\n")
        f.write("=" * 80 + "\n\n")

        # 기본 형식
        f.write("#[inline]\n")
        f.write("pub const fn is_safe_chars(c: char) -> bool {\n")
        f.write("    let code = c as u32;\n")
        f.write("    matches!(code,\n")

        for i, (start, end) in enumerate(ranges):
            if start == end:
                f.write(f"        0x{start:06X}")
            else:
                f.write(f"        0x{start:06X}..=0x{end:06X}")

            if i < len(ranges) - 1:
                f.write(" |")

            f.write("\n")

        f.write("    )\n")
        f.write("}\n\n")

        # 한 줄씩 형식 (더 읽기 쉬움)
        f.write("\n" + "=" * 80 + "\n")
        f.write("한 줄씩 형식 (복사하기 쉬운 버전)\n")
        f.write("=" * 80 + "\n\n")

        lines = []
        for start, end in ranges:
            if start == end:
                lines.append(f"0x{start:06X}")
            else:
                lines.append(f"0x{start:06X}..=0x{end:06X}")

        # 80자 제한으로 줄바꿈
        current_line = "        "
        for i, item in enumerate(lines):
            if len(current_line) + len(item) + 3 > 80 and current_line.strip():
                f.write(current_line + " |\n")
                current_line = "        "

            current_line += item
            if i < len(lines) - 1:
                current_line += " | "

        if current_line.strip():
            f.write(current_line + "\n")

        f.write("\n\n")

        # 통계
        f.write("=" * 80 + "\n")
        f.write("통계\n")
        f.write("=" * 80 + "\n")
        total_chars = sum(end - start + 1 for start, end in ranges)
        f.write(f"총 범위 수: {len(ranges)}\n")
        f.write(f"총 문자 수: {total_chars}\n")


def main():
    # 입력/출력 파일 경로
    csv_file = Path(__file__).parent / 'eztrans_translation_results.csv'
    rust_output = Path(__file__).parent / 'safe_chars_function.rs'
    text_output = Path(__file__).parent / 'safe_chars_matches.txt'

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

    # Rust 파일 생성
    print(f"\nRust 함수 생성 중: {rust_output}")
    generate_rust_matches_macro(ranges, rust_output)
    print("생성 완료!")

    # 텍스트 파일 생성
    print(f"텍스트 파일 생성 중: {text_output}")
    generate_compact_format(ranges, text_output)
    print("생성 완료!")

    print(f"\n생성된 파일:")
    print(f"  - {rust_output} (Rust 소스 코드)")
    print(f"  - {text_output} (다양한 형식)")

    # 미리보기
    print("\n" + "=" * 80)
    print("미리보기 (처음 10줄)")
    print("=" * 80)
    with open(rust_output, 'r', encoding='utf-8') as f:
        for i, line in enumerate(f):
            if i >= 15:
                print("...")
                break
            print(line, end='')


if __name__ == '__main__':
    main()
