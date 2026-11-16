# v0.1.1 검증 기록

검증일: 2026-11-16 (Asia/Seoul)

- cargo test --offline: 3 passed, 0 failed.
- cargo build --release --offline --target-dir target/distribution: 통과.
- 포터블 ZIP의 필수 파일, VERSION 0.1.1, 실행 파일과 빌드 원본의 SHA256 일치 확인.
- PDFium 및 서드파티 라이선스 포함.
- 설치 EXE: 최초 releases 출력에서 Inno Setup 리소스 갱신 오류 발생. target/release-publish 별도 출력에서 재컴파일 성공 후 releases로 복사.
- ZIP 및 설치 EXE SHA256 파일 일치 확인.
- 전체 GUI·실제 프린터·OCR·관리자 설치/제거 수동 검증은 수행하지 않음.
