import * as vscode from 'vscode';
import * as path from 'path';
import { SourcemapService } from '../services/sourcemapService';

/**
 * 调试同步控制器
 * 负责在调试时同步显示 Nu 和 Rust 代码
 */
export class DebugSyncController {
    private nuEditor: vscode.TextEditor | undefined;
    private rustEditor: vscode.TextEditor | undefined;
    private decorationType: vscode.TextEditorDecorationType;
    private disposables: vscode.Disposable[] = [];

    constructor(private sourcemapService: SourcemapService) {
        // 创建高亮装饰器
        this.decorationType = vscode.window.createTextEditorDecorationType({
            backgroundColor: new vscode.ThemeColor('editor.findMatchHighlightBackground'),
            border: '1px solid',
            borderColor: new vscode.ThemeColor('editor.findMatchHighlightBorder'),
            isWholeLine: true
        });
    }

    /**
     * 在调试启动时打开 Nu 文件
     */
    async openNuFileForDebug(nuFilePath: string, rsFilePath: string): Promise<void> {
        try {
            // 打开 Rust 文件（调试器会使用这个）
            const rustDoc = await vscode.workspace.openTextDocument(rsFilePath);
            this.rustEditor = await vscode.window.showTextDocument(rustDoc, {
                viewColumn: vscode.ViewColumn.One,
                preview: false
            });

            // 在右侧打开 Nu 文件
            const nuDoc = await vscode.workspace.openTextDocument(nuFilePath);
            this.nuEditor = await vscode.window.showTextDocument(nuDoc, {
                viewColumn: vscode.ViewColumn.Two,
                preview: false,
                preserveFocus: true // 保持焦点在 Rust 编辑器
            });

            // 加载 SourceMap
            const mapPath = rsFilePath + '.map';
            await this.sourcemapService.loadSourceMap(mapPath);

            vscode.window.showInformationMessage(
                'Debug view ready: Left=Rust (debugger), Right=Nu (source)',
                { modal: false }
            );
        } catch (error) {
            console.error('Failed to open Nu file for debug:', error);
            vscode.window.showWarningMessage(
                'Could not open Nu source file. Debugging will continue with Rust code only.'
            );
        }
    }

    /**
     * 启动调试同步监听
     */
    startSync(): void {
        // 监听调试会话的栈帧变化
        const debugTracker = vscode.debug.onDidChangeActiveStackItem(async (stackItem) => {
            if (stackItem) {
                await this.syncToStackFrame(stackItem);
            }
        });

        // 监听活动编辑器变化（用户在 Rust 编辑器中移动光标）
        const editorTracker = vscode.window.onDidChangeTextEditorSelection(async (event) => {
            if (event.textEditor === this.rustEditor && vscode.debug.activeDebugSession) {
                await this.syncRustToNu(event.textEditor.selection.active.line);
            }
        });

        this.disposables.push(debugTracker, editorTracker);
    }

    /**
     * 同步到调试栈帧
     */
    private async syncToStackFrame(stackItem: vscode.DebugThread | any): Promise<void> {
        // 检查是否是栈帧对象（具有 source 和 line 属性）
        const frame = stackItem as any;
        if (!frame.source || !frame.line) {
            return;
        }

        const rustLine = frame.line;
        await this.syncRustToNu(rustLine);
    }

    /**
     * 将 Rust 行号同步到 Nu 编辑器
     */
    private async syncRustToNu(rustLine: number): Promise<void> {
        if (!this.rustEditor || !this.nuEditor) {
            return;
        }

        const rsFilePath = this.rustEditor.document.fileName;
        const mapPath = rsFilePath + '.map';
        const mapping = this.sourcemapService.mapRustToNu(mapPath, rustLine + 1, 0);

        if (mapping) {
            const nuLine = mapping.nuLine;
            // 滚动到对应的 Nu 行
            const position = new vscode.Position(nuLine - 1, mapping.nuColumn);
            const range = new vscode.Range(position, position);

            this.nuEditor.revealRange(
                range,
                vscode.TextEditorRevealType.InCenterIfOutsideViewport
            );

            // 高亮对应的行
            this.nuEditor.setDecorations(this.decorationType, [range]);

            // 500ms 后清除高亮
            setTimeout(() => {
                if (this.nuEditor) {
                    this.nuEditor.setDecorations(this.decorationType, []);
                }
            }, 500);
        }
    }

    /**
     * 停止同步
     */
    stopSync(): void {
        this.disposables.forEach(d => d.dispose());
        this.disposables = [];
        
        if (this.nuEditor) {
            this.nuEditor.setDecorations(this.decorationType, []);
        }
    }

    /**
     * 清理资源
     */
    dispose(): void {
        this.stopSync();
        this.decorationType.dispose();
    }
}