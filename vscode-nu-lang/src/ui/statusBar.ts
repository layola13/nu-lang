import * as vscode from 'vscode';

export class StatusBarManager {
    private statusBarItem: vscode.StatusBarItem;
    private isCompiling: boolean = false;

    constructor() {
        this.statusBarItem = vscode.window.createStatusBarItem(
            vscode.StatusBarAlignment.Left,
            100
        );
        this.statusBarItem.command = 'nu-lang.toggleAutoCompile';
        this.updateStatusBar();
        this.statusBarItem.show();
    }

    setCompiling(isCompiling: boolean): void {
        this.isCompiling = isCompiling;
        this.updateStatusBar();
    }

    setAutoCompileEnabled(enabled: boolean): void {
        const config = vscode.workspace.getConfiguration('nu-lang');
        config.update('autoCompile', enabled, vscode.ConfigurationTarget.Global);
        this.updateStatusBar();
    }

    private updateStatusBar(): void {
        const config = vscode.workspace.getConfiguration('nu-lang');
        const autoCompileEnabled = config.get<boolean>('autoCompile', true);

        if (this.isCompiling) {
            this.statusBarItem.text = '$(sync~spin) Nu: Compiling...';
            this.statusBarItem.tooltip = 'Nu file is being compiled';
        } else if (autoCompileEnabled) {
            this.statusBarItem.text = '$(check) Nu: Auto';
            this.statusBarItem.tooltip = 'Auto-compile enabled (click to toggle)';
        } else {
            this.statusBarItem.text = '$(circle-slash) Nu: Manual';
            this.statusBarItem.tooltip = 'Auto-compile disabled (click to toggle)';
        }
    }

    showSuccess(message: string): void {
        this.statusBarItem.text = `$(check) Nu: ${message}`;
        setTimeout(() => this.updateStatusBar(), 3000);
    }

    showError(message: string): void {
        this.statusBarItem.text = `$(error) Nu: ${message}`;
        setTimeout(() => this.updateStatusBar(), 5000);
    }

    showWarning(message: string): void {
        this.statusBarItem.text = `$(warning) Nu: ${message}`;
        setTimeout(() => this.updateStatusBar(), 3000);
    }

    dispose(): void {
        this.statusBarItem.dispose();
    }
}