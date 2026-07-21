# npm 보안 마이너 업데이트 설계

## 목표

현재 npm 의존성의 알려진 취약점을 줄이되, 직접 의존성의 메이저 버전과 npm 기반 잠금 파일 체계는 유지한다.

## 범위

- `npm audit fix`를 `--force` 없이 실행한다.
- `package.json`에 선언된 semver 범위를 벗어나는 메이저 업데이트는 적용하지 않는다.
- npm과 `package-lock.json`을 계속 사용하며 pnpm 잠금 파일은 만들지 않는다.
- 변경 후 감사, 포맷 검사, 린트, TypeScript 검사, Vitest, 프로덕션 빌드를 실행한다.
- 같은 메이저 범위에서 해결되지 않는 취약점은 잔여 항목으로 보고한다.

## 변경 대상

- `package-lock.json`: 안전한 마이너·패치 및 전이 의존성 해상도 갱신
- `package.json`: `npm audit fix`가 기존 semver 범위 안에서 필요하다고 판단하는 경우에만 변경

## 제외 사항

- `npm audit fix --force`
- Vite, Vitest 등 직접 의존성의 메이저 업그레이드
- Rust 및 훅 구현 변경
