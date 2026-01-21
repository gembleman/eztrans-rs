#Ehnd_ReadMe
Ehnd(엔드) -- ezTrans Translation Enhance Plug-in
    v3.11

Author: 소쿠릿 (sokcuri)

http://sokcuri.neko.kr/
https://github.com/sokcuri/ehnd

 엔드는 이지트랜스 번역 품질의 향상과 사용자 사전 알고리즘 최적화를 도와주는 플러그인입니다.

 ＊ 설치 방법

  1) 다운받은 Ehnd 압축파일을 풀고 이지트랜스에 덮어씁니다 (J2KEngine.dll 덮어쓰기)
  2) 필터 및 사전을 설치합니다 (Ehnd 폴더 안에 압축을 풀면 됩니다)
  3) 평소대로 이지트랜스 모듈을 사용하는 툴을 사용합니다 (별도 프로그램을 실행할 필요 없음)


 ＊ 파일 구조

  - J2KEngine.dll : Ehnd 본체
  - J2KEngine.dlx
  |
  |-- Ehnd
  |   - ehnd_conf.ini : 엔드 설정 파일
  |   - Ehnd용 전처리/후처리 필터, 사전 파일
  |   | PreFilter*.txt : Ehnd 전처리 필터
  |   | PostFilter*.txt : Ehnd 후처리 필터
  |   | UserDict*.txt : Ehnd 사용자 사전
  |   | SkipLayer*.txt : Ehnd 스킵 레이어
  |
  |-- ETC
      - jkDicConverter.exe : 이지트랜스 사용자 사전 바이너리(UserDict.jk) 파일을 엔드에서 사용하는 사용자 사전으로 변환해 주는 도구입니다.
      - Ehnd 사용자 사전 부분 정리.txt
      - Ehnd_ChangeLog.txt
      - Ehnd_ReadMe.txt

           
 ＃ Ehnd 다운로드 파일에는 필터와 사전이 동봉되어 있지 않습니다. 별도의 사전/필터를 다운받아 Ehnd 폴더에 넣어주세요.


 ＃ 추천 텍스트 출력 프로그램

    아네모네 v1.01 (http://sokcuri.neko.kr/220108275884)


 ＃ 추천 사전

    꿀도르 사전 (http://blog.naver.com/waltherp38/220267098421)


 ＃ 사전 설치 방법

    꿀도르님의 "꿀도르 사전 설치 방법" - http://blog.naver.com/waltherp38/220286266694

    Foolmaker님의 "Ehnd 설명서 및 사전 설치 방법" - http://foolmaker.blog.me/30165239769


 ＊ 명령어 일람

  - 명령어는 INI 파일의 COMMAND_SWITCH가 ON으로 되어 있어야 사용이 가능합니다.
    /log, /command, /reload 명령어는 이 옵션에 상관없이 항상 작동합니다.

  - 명령어 사용시 설정 파일인 Ehnd\ehnd_conf.ini 파일에 저장됩니다
    단, /pre, /post, /userdic와 같은 일부 명령어는 사용하더라도 상태가 ini 파일에 저장되지 않습니다.

    /log : 로그 창을 엽니다
    /command : 명령어를 켜거나 끕니다
    /filelog : 파일 로그 작성을 켜거나 끕니다
    /log_detail : 세부 처리로그를 표시합니다
    /log_time : 처리내역의 소요시간을 로그에 표시합니다
    /log_skiplayer : 스킵레이어의 처리내역을 로그에 표시합니다
    /log_userdic : 사용자 사전의 처리내역을 로그에 표시합니다

  - 사용해도 INI에 상태가 저장되지 않는 명령어

    /reload : 필터와 사용자 사전을 다시 읽습니다
    /pre, /preon, /preoff : 전처리를 켜거나 끕니다
    /post, /poston, /postoff : 후처리를 켜거나 끕니다
    /dic, /dicon, /dicoff : 사용자 사전을 켜거나 끕니다.


 ＊ 작동 메커니즘

  - 후커 프로그램을 통해 이지트랜스로 번역문이 넘어오면 전처리 과정, 번역, 후처리 과정을 거치게 됩니다.
    전처리와 후처리는 EMCA 정규식이 지원됩니다.

  - 전처리/후처리 필터는 여러 파일로 분할이 가능하며, 모든 파일을 읽은 다음 차수가 낮은 순에서 높은 순으로, 동일 차수에서는 쓰여진 순서로 정렬합니다.
    (PreFilter*.txt, PostFilter*.txt)
    예) PreFilter1.txt, PreFilter2.txt, PostFilter1_기본.txt, PostFilter2_예비.txt ...

  - 사용자 사전도 여러 파일로 분할이 가능합니다. 별도의 정렬 과정은 거치지 않으며 가나다 순(Windows 탐색기 이름 정렬 순)으로 파일을 읽습니다.
    기본적으로 사용자 사전을 읽는 순서는 다음과 같습니다.
    예) (UserDict.jk 사용자 사전 사용 옵션이 켜져있는 경우) UserDict.jk → (아네모네가 켜져있는 경우) 아네모네 사용자 사전 AneDic.txt
    → UserDict*.txt (UserDict.txt → UserDict_게임1.txt → UserDict_단순.txt ...)

  - 필터를 스킵해주는 스킵 레이어 파일 또한 여러 파일로 분할이 가능합니다.
    (SkipLayer*.txt)
    예) SkipLayer.txt → SkipLayer_A.txt → SkipLayer_B.txt ...

  - 필터 및 사전 파일은 UTF-8 인코딩으로 적용됩니다. ANSI 또는 다른 인코딩으로 저장 시 정상적으로 작동하지 않습니다.

  - 설정 파일을 변경하고 저장하면 변경된 내용이 즉시 적용됩니다.

