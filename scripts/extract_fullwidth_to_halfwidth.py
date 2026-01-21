#!/usr/bin/env python3
"""
전각에서 반각으로 변환된 특수문자들을 추출하는 스크립트

이 스크립트는 default_translate_detection.csv 파일에서
전각(Full-width) 문자가 반각(Half-width) 문자로 변환된 경우를 찾아 출력합니다.
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


def extract_fullwidth_to_halfwidth_conversions(csv_file_path):
    """전각에서 반각으로 변환된 항목을 추출"""
    conversions = []

    with open(csv_file_path, 'r', encoding='utf-8-sig') as csvfile:
        reader = csv.DictReader(csvfile)

        for row in reader:
            code = row['Code']
            character = row['Character']
            translation = row['Translation']
            issue_type = row['IssueType']

            # 원본 문자가 전각이고 번역된 문자가 반각인 경우
            if character and translation and is_fullwidth_char(character) and is_halfwidth_char(translation):
                conversions.append({
                    'code': code,
                    'character': character,
                    'translation': translation,
                    'issue_type': issue_type,
                    'char_name': unicodedata.name(character, 'UNKNOWN'),
                    'trans_name': unicodedata.name(translation, 'UNKNOWN') if len(translation) == 1 else 'N/A'
                })

    return conversions


def print_conversions(conversions):
    """변환 결과를 출력"""
    print(f"전각 → 반각 변환된 문자 총 {len(conversions)}개 발견\n")
    print("=" * 120)
    print(f"{'코드':<12} {'전각문자':<6} {'반각문자':<6} {'이슈타입':<15} {'전각문자명':<40} {'반각문자명':<40}")
    print("=" * 120)

    for conv in conversions:
        print(f"{conv['code']:<12} {conv['character']:<6} {conv['translation']:<6} {conv['issue_type']:<15} "
              f"{conv['char_name']:<40} {conv['trans_name']:<40}")


def save_to_csv(conversions, output_file):
    """결과를 CSV 파일로 저장"""
    with open(output_file, 'w', encoding='utf-8-sig', newline='') as csvfile:
        fieldnames = ['code', 'character', 'translation', 'issue_type', 'char_name', 'trans_name']
        writer = csv.DictWriter(csvfile, fieldnames=fieldnames)

        writer.writeheader()
        writer.writerows(conversions)

    print(f"\n결과가 {output_file}에 저장되었습니다.")


def main():
    # 입력 파일 경로
    csv_file = Path(__file__).parent / 'default_translate_detection.csv'

    if not csv_file.exists():
        print(f"오류: {csv_file} 파일을 찾을 수 없습니다.")
        return

    # 전각 → 반각 변환 추출
    conversions = extract_fullwidth_to_halfwidth_conversions(csv_file)

    # 결과 출력
    print_conversions(conversions)

    # CSV로 저장
    output_file = Path(__file__).parent / 'fullwidth_to_halfwidth_conversions.csv'
    save_to_csv(conversions, output_file)

    # 카테고리별 통계
    print("\n" + "=" * 120)
    print("카테고리별 통계:")
    print("=" * 120)

    categories = {
        '숫자': lambda c: c['translation'].isdigit(),
        '영문 대문자': lambda c: c['translation'].isupper() and c['translation'].isalpha(),
        '영문 소문자': lambda c: c['translation'].islower() and c['translation'].isalpha(),
        '특수기호': lambda c: not c['translation'].isalnum() and len(c['translation']) == 1,
        '기타': lambda c: True
    }

    for category, condition in categories.items():
        count = sum(1 for c in conversions if condition(c))
        if count > 0:
            print(f"{category}: {count}개")


if __name__ == '__main__':
    main()
