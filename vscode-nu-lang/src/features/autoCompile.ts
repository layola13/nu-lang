import * as vscode from 'vscode';
import * as path from 'path';
import { ConversionService } from '../services/conversionService';
import { SourcemapService } from '../services/sourcemapService';
import { CargoService } from '../services/cargoService';
import { StatusBarManager } from '../ui/statusBar';

export class AutoCompileFeature {
    private fileWatcher: vscode.FileSystemWatcher | null = null;
    private isCompiling: boolean = false;
    private compilationQueue: Set<string> = new Set();

    constructor(
        private conversionService: ConversionService,
        private sourcemapService: SourcemapService,
        private cargoService: CargoService,
        private statusBar: StatusBarManager,
        private onCompilationComplete: (nuFilePath: string, success: boolean) => void
    ) {}

    activate(context: vscode.ExtensionContext): void {
        // 监听 .nu 文件的保存事件
        const saveListener = vscode.workspace.onDidSaveTextDocument(async (document) => {
            if (document.languageId === 'nu' && this.isAutoCompileEnabled()) {
                await this.compileFile(document.uri.fsPath);
            }
        });

        // 创建文件监视器（备用方案）
        this.fileWatcher = vscode.workspace.createFileSystemWatcher('**/*.nu');
        
        this.fileWatcher.onDidChange(async (uri) => {
            if (this.isAutoCompileEnabled()) {
                // 延迟执行，避免频繁编译
                setTimeout(() => {
                    if (!this.compilationQueue.has(uri.fsPath)) {
                        this.compileFile(uri.fsPath);
                    }
                }, 500);
            }
        });

        context.subscriptions.push(saveListener, this.fileWatcher);
    }

    async compileFile(nuFilePath: string): Promise<boolean> {
        // 防止重复编译
        if (this.compilationQueue.has(nuFilePath)) {
            return false;
        }

        this.compilationQueue.add(nuFilePath);
        this.isCompiling = true;
        this.statusBar.setCompiling(true);

        try {
            // 显示进度
            return await vscode.window.withProgress(
                {
                    location: vscode.ProgressLocation.Notification,
                    title: 'Nu Compilation',
                    cancellable: false
                },
                async (progress) => {
                    progress.report({ message: 'Converting Nu to Rust...', increment: 0 });

                    // 步骤 1: 转换 Nu 到 Rust
                    const conversionResult = await this.conversionService.convertFile(nuFilePath);
                    
                    if (!conversionResult.success) {
                        vscode.window.showErrorMessage(
                            `Nu compilation failed: ${conversionResult.error}`
                        );
                        this.statusBar.showError('Compilation failed');
                        this.onCompilationComplete(nuFilePath, false);
                        return false;
                    }

                    progress.report({ message: 'Loading source map...', increment: 30 });

                    // 步骤 2: 加载 SourceMap
                    if (conversionResult.mapPath) {
                        await this.sourcemapService.loadSourceMap(conversionResult.mapPath);
                    }

                    // 步骤 3: 运行 cargo check（如果启用）
                    if (this.isAutoCheckEnabled() && conversionResult.outputPath) {
                        progress.report({ message: 'Running cargo check...', increment: 50 });
                        
                        const checkResult = await this.cargoService.checkFile(conversionResult.outputPath);
                        
                        if (!checkResult.success) {
                            this.statusBar.showWarning(`${checkResult.errors.length} errors`);
                        } else if (checkResult.warnings.length > 0) {
                            this.statusBar.showWarning(`${checkResult.warnings.length} warnings`);
                        } else {
                            this.statusBar.showSuccess('Compiled successfully');
                        }

                        this.onCompilationComplete(nuFilePath, checkResult.success);
                        progress.report({ message: 'Complete', increment: 100 });
                        return checkResult.success;
                    } else {
                        this.statusBar.showSuccess('Compiled successfully');
                        this.onCompilationComplete(nuFilePath, true);
                        progress.report({ message: 'Complete', increment: 100 });
                        return true;
                    }
                }
            );
        } catch (error: any) {
            vscode.window.showErrorMessage(`Compilation error: ${error.message}`);
            this.statusBar.showError('Compilation error');
            this.onCompilationComplete(nuFilePath, false);
            return false;
        } finally {
            this.compilationQueue.delete(nuFilePath);
            this.isCompiling = false;
            this.statusBar.setCompiling(false);
        }
    }

    async compileCurrentFile(): Promise<void> {
        const editor = vscode.window.activeTextEditor;
        
        if (!editor || editor.document.languageId !== 'nu') {
            vscode.window.showWarningMessage('Please open a .nu file first');
            return;
        }

        // 如果文件未保存，先保存
        if (editor.document.isDirty) {
            await editor.document.save();
        }

        await this.compileFile(editor.document.uri.fsPath);
    }

    private isAutoCompileEnabled(): boolean {
        const config = vscode.workspace.getConfiguration('nu-lang');
        return config.get<boolean>('autoCompile', true);
    }

    private isAutoCheckEnabled(): boolean {
        const config = vscode.workspace.getConfiguration('nu-lang');
        return config.get<boolean>('autoCheck', true);
    }

    toggleAutoCompile(): void {
        const config = vscode.workspace.getConfiguration('nu-lang');
        const currentValue = config.get<boolean>('autoCompile', true);
        config.update('autoCompile', !currentValue, vscode.ConfigurationTarget.Global);
        
        const newState = !currentValue ? 'enabled' : 'disabled';
        vscode.window.showInformationMessage(`Nu auto-compile ${newState}`);
        this.statusBar.setAutoCompileEnabled(!currentValue);
    }

    dispose(): void {
        if (this.fileWatcher) {
            this.fileWatcher.dispose();
        }
    }
}