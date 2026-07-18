import { Component, ReactNode } from "react";

type AppErrorBoundaryProps = {
  children: ReactNode;
  onReload?: () => void;
};

type AppErrorBoundaryState = {
  hasError: boolean;
};

export class AppErrorBoundary extends Component<AppErrorBoundaryProps, AppErrorBoundaryState> {
  state: AppErrorBoundaryState = { hasError: false };

  static getDerivedStateFromError(): AppErrorBoundaryState {
    return { hasError: true };
  }

  render() {
    if (!this.state.hasError) return this.props.children;

    return (
      <main role="alert">
        <h1>Djeeta MOD</h1>
        <p>화면을 표시할 수 없습니다</p>
        <p>앱을 다시 불러온 뒤에도 문제가 계속되면 전투 기록과 설정 파일을 보존하고 오류를 보고해 주세요.</p>
        <button type="button" onClick={this.props.onReload ?? (() => window.location.reload())}>
          다시 불러오기
        </button>
      </main>
    );
  }
}
