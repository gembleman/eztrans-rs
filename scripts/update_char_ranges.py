#!/usr/bin/env python3
"""
char_ranges.rs의 is_safe_chars 함수에서
matches가 TRUE인 범위를 제외하는 스크립트
"""

import csv
import re
from pathlib import Path
from typing import List, Tuple, Set


def parse_unicode_code(code: str) -> int:
    """유니코드 코드 포인트를 파싱"""
    if code.startswith('U+'):
        return int(code[2:], 16)
    return int(code, 16)


def extract_matched_unicode_ranges(csv_file_path: Path) -> List[int]:
    """matches가 TRUE인 유니코드 코드 포인트들을 추출"""
    matched_codes = []

    with open(csv_file_path, 'r', encoding='utf-8-sig') as csvfile:
        reader = csv.DictReader(csvfile)

        for row in reader:
            matches = row.get('matches', '').strip().lower()
            if matches in ['true', '1', 'yes']:
                code = row.get('code', '').strip()
                if code:
                    try:
                        code_point = parse_unicode_code(code)
                        matched_codes.append(code_point)
                    except ValueError:
                        pass

    return matched_codes


def find_unicode_ranges(code_points: List[int]) -> List[Tuple[int, int]]:
    """연속된 유니코드 코드 포인트들을 범위로 그룹화"""
    if not code_points:
        return []

    sorted_points = sorted(set(code_points))
    ranges = []
    start = sorted_points[0]
    end = sorted_points[0]

    for point in sorted_points[1:]:
        if point == end + 1:
            end = point
        else:
            ranges.append((start, end))
            start = point
            end = point

    ranges.append((start, end))
    return ranges


def parse_range_line(line: str) -> List[Tuple[int, int]]:
    """범위 라인을 파싱하여 (시작, 끝) 튜플 리스트 반환"""
    ranges = []

    # 0x000020..=0x00007E 또는 0x000020 형식 찾기
    pattern = r'0x([0-9A-Fa-f]+)(?:\.\.=0x([0-9A-Fa-f]+))?'
    matches = re.findall(pattern, line)

    for match in matches:
        start = int(match[0], 16)
        end = int(match[1], 16) if match[1] else start
        ranges.append((start, end))

    return ranges


def is_point_in_ranges(point: int, ranges: List[Tuple[int, int]]) -> bool:
    """포인트가 범위 리스트에 포함되는지 확인"""
    for start, end in ranges:
        if start <= point <= end:
            return True
    return False


def subtract_ranges(original_ranges: List[Tuple[int, int]],
                   exclude_ranges: List[Tuple[int, int]]) -> List[Tuple[int, int]]:
    """원본 범위에서 제외 범위를 뺀 새로운 범위 생성"""
    # 제외할 포인트 집합 생성
    exclude_points = set()
    for start, end in exclude_ranges:
        for point in range(start, end + 1):
            exclude_points.add(point)

    # 원본 범위의 모든 포인트 생성
    result_points = []
    for start, end in original_ranges:
        for point in range(start, end + 1):
            if point not in exclude_points:
                result_points.append(point)

    # 새로운 범위로 그룹화
    return find_unicode_ranges(result_points)


def read_is_safe_chars_function(file_path: Path) -> Tuple[List[str], List[str], List[str]]:
    """is_safe_chars 함수를 읽고 헤더, 범위 라인들, 푸터로 분리"""
    with open(file_path, 'r', encoding='utf-8') as f:
        lines = f.readlines()

    # 함수 시작과 끝 찾기
    func_start = -1
    matches_start = -1
    matches_end = -1

    for i, line in enumerate(lines):
        if 'pub const fn is_safe_chars' in line:
            func_start = i
        elif func_start != -1 and matches_start == -1 and 'matches!(code,' in line:
            matches_start = i + 1  # matches!( 다음 줄부터
        elif matches_start != -1 and matches_end == -1 and ')' in line and '|' not in line:
            matches_end = i
            break

    if func_start == -1 or matches_start == -1 or matches_end == -1:
        raise ValueError("is_safe_chars 함수를 찾을 수 없습니다.")

    # 헤더 (함수 시작부터 matches!( 까지)
    header = lines[:matches_start]

    # 범위 라인들
    range_lines = lines[matches_start:matches_end]

    # 푸터 (함수 끝부터 파일 끝까지)
    footer = lines[matches_end:]

    return header, range_lines, footer


def extract_ranges_from_lines(range_lines: List[str]) -> List[Tuple[int, int]]:
    """범위 라인들에서 모든 범위 추출"""
    all_ranges = []

    for line in range_lines:
        ranges = parse_range_line(line)
        all_ranges.extend(ranges)

    return all_ranges


def format_range(start: int, end: int) -> str:
    """범위를 Rust 형식 문자열로 변환"""
    if start == end:
        return f"0x{start:06X}"
    else:
        return f"0x{start:06X}..=0x{end:06X}"


def write_updated_function(output_file: Path, header: List[str],
                          new_ranges: List[Tuple[int, int]], footer: List[str]):
    """업데이트된 함수를 파일로 작성"""
    with open(output_file, 'w', encoding='utf-8') as f:
        # 헤더 작성
        f.writelines(header)

        # 새로운 범위 작성
        for i, (start, end) in enumerate(new_ranges):
            range_str = format_range(start, end)

            if i < len(new_ranges) - 1:
                f.write(f"        {range_str} |\n")
            else:
                f.write(f"        {range_str}\n")

        # 푸터 작성
        f.writelines(footer)


def main():
    base_path = Path(__file__).parent
    char_ranges_file = base_path / 'src' / 'char_ranges.rs'
    csv_file = base_path / 'eztrans_translation_results.csv'
    backup_file = base_path / 'src' / 'char_ranges.rs.backup'
    output_file = base_path / 'src' / 'char_ranges.rs.new'

    # 파일 존재 확인
    if not char_ranges_file.exists():
        print(f"오류: {char_ranges_file} 파일을 찾을 수 없습니다.")
        return

    if not csv_file.exists():
        print(f"오류: {csv_file} 파일을 찾을 수 없습니다.")
        return

    print("=" * 80)
    print("char_ranges.rs 업데이트 스크립트")
    print("=" * 80)

    # 1. CSV에서 제외할 범위 추출
    print("\n1. CSV에서 제외할 범위 추출 중...")
    matched_codes = extract_matched_unicode_ranges(csv_file)
    exclude_ranges = find_unicode_ranges(matched_codes)
    print(f"   제외할 문자: {len(matched_codes)}개")
    print(f"   제외할 범위: {len(exclude_ranges)}개")

    # 2. 기존 is_safe_chars 함수 읽기
    print("\n2. 기존 is_safe_chars 함수 읽기 중...")
    header, range_lines, footer = read_is_safe_chars_function(char_ranges_file)
    print(f"   헤더: {len(header)}줄")
    print(f"   범위: {len(range_lines)}줄")
    print(f"   푸터: {len(footer)}줄")

    # 3. 기존 범위 추출
    print("\n3. 기존 범위 파싱 중...")
    original_ranges = extract_ranges_from_lines(range_lines)
    print(f"   기존 범위 수: {len(original_ranges)}개")

    # 4. 범위 빼기
    print("\n4. 새로운 범위 계산 중...")
    new_ranges = subtract_ranges(original_ranges, exclude_ranges)
    print(f"   새로운 범위 수: {len(new_ranges)}개")

    # 통계
    original_count = sum(end - start + 1 for start, end in original_ranges)
    exclude_count = sum(end - start + 1 for start, end in exclude_ranges)
    new_count = sum(end - start + 1 for start, end in new_ranges)

    print(f"\n   통계:")
    print(f"   - 기존 문자 수: {original_count}개")
    print(f"   - 제외할 문자 수: {exclude_count}개")
    print(f"   - 남은 문자 수: {new_count}개")
    print(f"   - 실제 제거된 문자 수: {original_count - new_count}개")

    # 5. 백업 생성
    print(f"\n5. 백업 파일 생성 중: {backup_file}")
    import shutil
    shutil.copy2(char_ranges_file, backup_file)
    print("   백업 완료!")

    # 6. 새 파일 작성
    print(f"\n6. 새 파일 작성 중: {output_file}")
    write_updated_function(output_file, header, new_ranges, footer)
    print("   작성 완료!")

    # 7. 미리보기
    print("\n7. 변경 사항 미리보기 (처음 10개 범위):")
    print("   " + "=" * 76)
    for i, (start, end) in enumerate(new_ranges[:10]):
        range_str = format_range(start, end)
        print(f"   {range_str}")
    if len(new_ranges) > 10:
        print(f"   ... (그 외 {len(new_ranges) - 10}개)")
    print("   " + "=" * 76)

    print("\n완료!")
    print("\n다음 단계:")
    print(f"1. 새 파일 확인: {output_file}")
    print(f"2. 문제가 없으면 다음 명령 실행:")
    print(f"   mv \"{output_file}\" \"{char_ranges_file}\"")
    print(f"3. 원래대로 되돌리려면:")
    print(f"   mv \"{backup_file}\" \"{char_ranges_file}\"")


if __name__ == '__main__':
    main()
