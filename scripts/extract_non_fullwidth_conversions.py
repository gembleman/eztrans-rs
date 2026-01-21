#!/usr/bin/env python3
"""
전각에서 반각으로 변환되지 않은 문자들을 추출하는 스크립트

이 스크립트는 default_translate_detection.csv 파일에서
전각 → 반각 변환이 아닌 다른 모든 변환들을 추출합니다.
"""

import csv
import unicodedata
from pathlib import Path


def is_fullwidth_char(char):
    """문자가 전각인지 확인"""
    if not char or len(char) != 1:
        return False

    code_point = ord(char)

    # 전각 영숫자 및 기호 범위 (FF00-FF5F)
    if 0xFF00 <= code_point <= 0xFF5F:
        return True

    # 전각 반각 가타카나 (FF61-FF9F)
    if 0xFF61 <= code_point <= 0xFF9F:
        return True

    # CJK 공백 (3000)
    if code_point == 0x3000:
        return True

    # CJK 기호와 문장부호 (3001-303F)
    if 0x3001 <= code_point <= 0x303F:
        return True

    # 기타 전각 문자들
    fullwidth_ranges = [
        (0x2E80, 0x2EFF),  # CJK Radicals Supplement
        (0x3040, 0x309F),  # Hiragana
        (0x30A0, 0x30FF),  # Katakana
        (0x3100, 0x312F),  # Bopomofo
        (0x3200, 0x32FF),  # Enclosed CJK Letters and Months
        (0x3300, 0x33FF),  # CJK Compatibility
    ]

    for start, end in fullwidth_ranges:
        if start <= code_point <= end:
            return True

    return False


def is_halfwidth_char(char):
    """문자가 반각인지 확인"""
    if not char or len(char) != 1:
        return False

    code_point = ord(char)

    # ASCII 범위 (0020-007E)
    if 0x0020 <= code_point <= 0x007E:
        return True

    # 반각 가타카나 (FF61-FF9F)
    if 0xFF61 <= code_point <= 0xFF9F:
        return True

    return False


def extract_non_fullwidth_conversions(csv_file_path):
    """전각에서 반각으로 변환되지 않은 항목을 추출"""
    conversions = []

    with open(csv_file_path, 'r', encoding='utf-8-sig') as csvfile:
        reader = csv.DictReader(csvfile)

        for row in reader:
            code = row['Code']
            character = row['Character']
            translation = row['Translation']
            issue_type = row['IssueType']

            # 전각 → 반각 변환이 아닌 경우만 포함
            is_fullwidth_to_halfwidth = (
                character and
                translation and
                len(character) == 1 and
                len(translation) == 1 and
                is_fullwidth_char(character) and
                is_halfwidth_char(translation)
            )

            if not is_fullwidth_to_halfwidth:
                char_name = 'N/A'
                trans_name = 'N/A'

                try:
                    if character and len(character) == 1:
                        char_name = unicodedata.name(character, 'UNKNOWN')
                except:
                    char_name = 'ERROR'

                try:
                    if translation and len(translation) == 1:
                        trans_name = unicodedata.name(translation, 'UNKNOWN')
                except:
                    trans_name = 'ERROR'

                conversions.append({
                    'code': code,
                    'character': character,
                    'translation': translation,
                    'issue_type': issue_type,
                    'char_name': char_name,
                    'trans_name': trans_name
                })

    return conversions


def categorize_conversion(conv):
    """변환 타입을 분류"""
    char = conv['character']
    trans = conv['translation']

    # 빈 문자열로 변환
    if not trans:
        return '삭제됨 (빈 문자열)'

    # 빈 문자열에서 변환
    if not char:
        return '추가됨 (빈 문자열에서)'

    # 멀티바이트 문자
    if len(char) > 1 or len(trans) > 1:
        return '멀티바이트 변환'

    # 한자 관련
    if '\u4e00' <= char <= '\u9fff':
        if trans.isdigit():
            return '한자 숫자 → 아라비아 숫자'
        else:
            return '한자 → 기타'

    # 히라가나/가타카나
    if '\u3040' <= char <= '\u309f' or '\u30a0' <= char <= '\u30ff':
        if not trans:
            return '히라가나/가타카나 삭제'
        else:
            return '히라가나/가타카나 → 기타'

    # 특수문자 변환
    if not char.isalnum() and not trans.isalnum():
        return '특수문자 → 특수문자'

    # 알파벳 변환
    if char.isalpha() and trans.isalpha():
        return '알파벳 → 알파벳'

    # 문자 → 숫자
    if not char.isdigit() and trans.isdigit():
        return '문자 → 숫자'

    # 숫자 → 문자
    if char.isdigit() and not trans.isdigit():
        return '숫자 → 문자'

    return '기타 변환'


def print_summary(conversions):
    """요약 정보를 출력"""
    print(f"전각 → 반각 변환을 제외한 문자 총 {len(conversions)}개 발견\n")

    # 카테고리별 통계
    categories = {}
    for conv in conversions:
        category = categorize_conversion(conv)
        categories[category] = categories.get(category, 0) + 1

    print("=" * 80)
    print("카테고리별 통계:")
    print("=" * 80)
    for category, count in sorted(categories.items(), key=lambda x: -x[1]):
        print(f"{category}: {count}개")

    # IssueType별 통계
    issue_types = {}
    for conv in conversions:
        issue_type = conv['issue_type']
        issue_types[issue_type] = issue_types.get(issue_type, 0) + 1

    print("\n" + "=" * 80)
    print("IssueType별 통계:")
    print("=" * 80)
    for issue_type, count in sorted(issue_types.items(), key=lambda x: -x[1]):
        print(f"{issue_type}: {count}개")


def save_to_csv(conversions, output_file):
    """결과를 CSV 파일로 저장"""
    # 카테고리 정보 추가
    for conv in conversions:
        conv['category'] = categorize_conversion(conv)

    with open(output_file, 'w', encoding='utf-8-sig', newline='') as csvfile:
        fieldnames = ['code', 'character', 'translation', 'issue_type', 'category', 'char_name', 'trans_name']
        writer = csv.DictWriter(csvfile, fieldnames=fieldnames)

        writer.writeheader()
        writer.writerows(conversions)

    print(f"\n결과가 {output_file}에 저장되었습니다.")


def print_sample_data(conversions, sample_size=20):
    """샘플 데이터 출력"""
    print(f"\n샘플 데이터 (처음 {sample_size}개):")
    print("=" * 120)
    print(f"{'코드':<12} {'원본':<8} {'변환':<8} {'타입':<15} {'카테고리':<25}")
    print("=" * 120)

    for conv in conversions[:sample_size]:
        char_display = conv['character'] if conv['character'] else '(empty)'
        trans_display = conv['translation'] if conv['translation'] else '(empty)'
        category = categorize_conversion(conv)
        issue_type = conv['issue_type'] if conv['issue_type'] else 'N/A'

        print(f"{conv['code']:<12} {char_display:<8} {trans_display:<8} {issue_type:<15} {category:<25}")


def main():
    # 입력 파일 경로
    csv_file = Path(__file__).parent / 'default_translate_detection.csv'

    if not csv_file.exists():
        print(f"오류: {csv_file} 파일을 찾을 수 없습니다.")
        return

    # 전각 → 반각이 아닌 변환 추출
    conversions = extract_non_fullwidth_conversions(csv_file)

    # 요약 출력
    print_summary(conversions)

    # 샘플 데이터 출력
    print_sample_data(conversions, sample_size=30)

    # CSV로 저장
    output_file = Path(__file__).parent / 'non_fullwidth_conversions.csv'
    save_to_csv(conversions, output_file)


if __name__ == '__main__':
    main()
