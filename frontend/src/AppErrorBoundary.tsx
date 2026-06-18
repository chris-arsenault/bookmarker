import { Component, type ReactNode } from "react";
import { config } from "./config";

type AppErrorBoundaryProps = {
  children: ReactNode;
};

type AppErrorBoundaryState = {
  message: string | null;
};

export class AppErrorBoundary extends Component<AppErrorBoundaryProps, AppErrorBoundaryState> {
  state: AppErrorBoundaryState = { message: null };

  static getDerivedStateFromError(error: unknown): AppErrorBoundaryState {
    return { message: errorMessage(error) };
  }

  render() {
    if (this.state.message) {
      return <AppCrashPanel message={this.state.message} />;
    }
    return this.props.children;
  }
}

function AppCrashPanel({ message }: { message: string }) {
  return (
    <main className="app-shell app-shell-centered">
      <section className="empty-state">
        <p className="eyebrow">{config.productName}</p>
        <h1>Application error</h1>
        <p>{message}</p>
      </section>
    </main>
  );
}

function errorMessage(error: unknown) {
  return error instanceof Error && error.message.trim()
    ? error.message
    : "The interface crashed while rendering.";
}
