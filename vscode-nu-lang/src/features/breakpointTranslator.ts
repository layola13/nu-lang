import * as vscode from 'vscode';
import * as path from 'path';
import { SourcemapService } from '../services/sourcemapService';

/**
 * 断点转换器
 * 将 Nu 文件中的断点自动映射到对应的 Rust 文件
 */
export class BreakpointTranslator {
    private nuBreakpoints: Map<string, vscode.SourceBreakpoint[]> = new Map();
    private rustBreakpoints: Map<string, vscode.SourceBreakpoint[]> = new Map();
    private disposables: vscode.Disposable[] = [];

    constructor(private sourcemapService: SourcemapService) {
        this.setupBreakpointListener();
    }

    /**
     * 监听断点变化
     */
    private setupBreakpointListener(): void {
        // 监听断点变化事件
        const breakpointListener = vscode.debug.onDidChangeBreakpoints(async (event) => {
            // 处理新增的断点
            for (const bp of event.added) {
                if (bp instanceof vscode.SourceBreakpoint) {
                    await this.handleBreakpointAdded(bp);
                }
            }

            // 处理移除的断点
            for (const bp of event.removed) {
                if (bp instanceof vscode.SourceBreakpoint) {
                    await this.handleBreakpointRemoved(bp);
                }
            }

            // 处理修改的断点
            for (const bp of event.changed) {
                if (bp instanceof vscode.SourceBreakpoint) {
                    await this.handleBreakpointChanged(bp);
                }
            }
        });

        this.disposables.push(breakpointListener);
    }

    /**
     * 处理新增断点
     */
    private async handleBreakpointAdded(breakpoint: vscode.SourceBreakpoint): Promise<void> {
        const location = breakpoint.location;
        const filePath = location.uri.fsPath;

        // 只处理 .nu 文件的断点
        if (!filePath.endsWith('.nu')) {
            return;
        }

        console.log(`[BreakpointTranslator] Nu breakpoint added at ${filePath}:${location.range.start.line + 1}`);

        // 获取对应的 Rust 文件
        const rsFilePath = filePath.replace(/\.nu$/, '.rs');
        const mapPath = rsFilePath + '.map';

        // 加载 SourceMap
        await this.sourcemapService.loadSourceMap(mapPath);

        // 映射断点位置
        const nuLine = location.range.start.line + 1; // VSCode 行号从 0 开始，SourceMap 从 1 开始
        const mapping = this.sourcemapService.mapNuToRust(mapPath, nuLine, 0);

        if (!mapping) {
            vscode.window.showWarningMessage(
                `Cannot map Nu breakpoint at line ${nuLine}. SourceMap may be outdated.`
            );
            return;
        }

        console.log(`[BreakpointTranslator] Mapped to Rust ${rsFilePath}:${mapping.rsLine}`);

        // 在 Rust 文件中设置对应的断点
        const rustUri = vscode.Uri.file(rsFilePath);
        const rustPosition = new vscode.Position(mapping.rsLine - 1, mapping.rsColumn);
        const rustLocation = new vscode.Location(rustUri, rustPosition);
        const rustBreakpoint = new vscode.SourceBreakpoint(rustLocation, breakpoint.enabled);

        // 保存映射关系
        const nuBps = this.nuBreakpoints.get(filePath) || [];
        nuBps.push(breakpoint);
        this.nuBreakpoints.set(filePath, nuBps);

        const rustBps = this.rustBreakpoints.get(rsFilePath) || [];
        rustBps.push(rustBreakpoint);
        this.rustBreakpoints.set(rsFilePath, rustBps);

        // 添加 Rust 断点到调试器
        const allBreakpoints = vscode.debug.breakpoints;
        vscode.debug.addBreakpoints([rustBreakpoint]);

        // 提示用户
        vscode.window.showInformationMessage(
            `Nu breakpoint at line ${nuLine} → Rust line ${mapping.rsLine}`,
            { modal: false }
        );
    }

    /**
     * 处理移除断点
     */
    private async handleBreakpointRemoved(breakpoint: vscode.SourceBreakpoint): Promise<void> {
        const location = breakpoint.location;
        const filePath = location.uri.fsPath;

        if (!filePath.endsWith('.nu')) {
            return;
        }

        console.log(`[BreakpointTranslator] Nu breakpoint removed at ${filePath}:${location.range.start.line + 1}`);

        // 查找并移除对应的 Rust 断点
        const rsFilePath = filePath.replace(/\.nu$/, '.rs');
        const rustBps = this.rustBreakpoints.get(rsFilePath) || [];

        // 移除所有相关的 Rust 断点
        if (rustBps.length > 0) {
            vscode.debug.removeBreakpoints(rustBps);
            this.rustBreakpoints.delete(rsFilePath);
        }

        // 清除 Nu 断点记录
        this.nuBreakpoints.delete(filePath);
    }

    /**
     * 处理修改断点
     */
    private async handleBreakpointChanged(breakpoint: vscode.SourceBreakpoint): Promise<void> {
        // 先移除旧断点，再添加新断点
        await this.handleBreakpointRemoved(breakpoint);
        await this.handleBreakpointAdded(breakpoint);
    }

    /**
     * 启动时同步现有断点
     */
    async syncExistingBreakpoints(): Promise<void> {
        const allBreakpoints = vscode.debug.breakpoints;

        for (const bp of allBreakpoints) {
            if (bp instanceof vscode.SourceBreakpoint) {
                const filePath = bp.location.uri.fsPath;
                if (filePath.endsWith('.nu')) {
                    await this.handleBreakpointAdded(bp);
                }
            }
        }
    }

    /**
     * 清理所有映射的断点
     */
    clearAllMappedBreakpoints(): void {
        // 移除所有 Rust 断点
        for (const rustBps of this.rustBreakpoints.values()) {
            vscode.debug.removeBreakpoints(rustBps);
        }

        this.nuBreakpoints.clear();
        this.rustBreakpoints.clear();
    }

    /**
     * 清理资源
     */
    dispose(): void {
        this.clearAllMappedBreakpoints();
        this.disposables.forEach(d => d.dispose());
        this.disposables = [];
    }
}