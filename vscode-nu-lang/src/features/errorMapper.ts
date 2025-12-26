import * as vscode from 'vscode';
import * as path from 'path';
import { CargoService, CargoError, CargoSpan } from '../services/cargoService';
import { SourcemapService } from '../services/sourcemapService';

export class ErrorMapperFeature {
    private diagnosticCollection: vscode.DiagnosticCollection;
    private diagnosticsCache: Map<string, vscode.Diagnostic[]> = new Map();

    constructor(
        private cargoService: CargoService,
        private sourcemapService: SourcemapService
    ) {
        this.diagnosticCollection = vscode.languages.createDiagnosticCollection('nu');
    }

    async mapErrorsForFile(nuFilePath: string, rsFilePath: string): Promise<void> {
        // 清除旧的诊断信息
        this.clearDiagnostics(nuFilePath);

        // 运行 cargo check
        const checkResult = await this.cargoService.checkFile(rsFilePath);
        
        // 过滤出与当前文件相关的错误
        const filteredResult = this.cargoService.filterErrorsForFile(checkResult, rsFilePath);

        // 加载 SourceMap
        const mapPath = this.getMapPath(nuFilePath);
        await this.sourcemapService.loadSourceMap(mapPath);

        // 映射错误到 .nu 文件
        const diagnostics: vscode.Diagnostic[] = [];

        // 处理错误
        for (const error of filteredResult.errors) {
            const mapped = await this.mapError(error, nuFilePath, rsFilePath, mapPath);
            diagnostics.push(...mapped);
        }

        // 处理警告
        for (const warning of filteredResult.warnings) {
            const mapped = await this.mapError(warning, nuFilePath, rsFilePath, mapPath);
            diagnostics.push(...mapped);
        }

        // 更新诊断信息
        this.updateDiagnostics(nuFilePath, diagnostics);
    }

    private async mapError(
        error: CargoError,
        nuFilePath: string,
        rsFilePath: string,
        mapPath: string
    ): Promise<vscode.Diagnostic[]> {
        const diagnostics: vscode.Diagnostic[] = [];

        for (const span of error.spans) {
            // 只处理主要的 span
            if (!span.is_primary) {
                continue;
            }

            // 只处理当前 Rust 文件的错误
            if (path.normalize(span.file_name) !== path.normalize(rsFilePath)) {
                continue;
            }

            // 使用 SourceMap 映射位置
            const mapping = this.sourcemapService.mapRustToNu(
                mapPath,
                span.line_start,
                span.column_start
            );

            if (!mapping) {
                // 如果找不到映射，跳过此错误
                continue;
            }

            // 创建诊断信息
            const range = new vscode.Range(
                new vscode.Position(mapping.nuLine - 1, mapping.nuColumn),
                new vscode.Position(mapping.nuLine - 1, mapping.nuColumn + (span.column_end - span.column_start))
            );

            const diagnostic = new vscode.Diagnostic(
                range,
                this.formatErrorMessage(error),
                this.getSeverity(error.level)
            );

            diagnostic.source = 'nu-lang';
            
            if (error.code) {
                diagnostic.code = error.code.code;
            }

            // 添加相关信息
            if (error.children && error.children.length > 0) {
                const relatedInformation: vscode.DiagnosticRelatedInformation[] = [];
                
                for (const child of error.children) {
                    if (child.spans && child.spans.length > 0) {
                        const childSpan = child.spans[0];
                        const childMapping = this.sourcemapService.mapRustToNu(
                            mapPath,
                            childSpan.line_start,
                            childSpan.column_start
                        );

                        if (childMapping) {
                            const location = new vscode.Location(
                                vscode.Uri.file(nuFilePath),
                                new vscode.Position(childMapping.nuLine - 1, childMapping.nuColumn)
                            );

                            relatedInformation.push(
                                new vscode.DiagnosticRelatedInformation(
                                    location,
                                    child.message
                                )
                            );
                        }
                    }
                }

                diagnostic.relatedInformation = relatedInformation;
            }

            diagnostics.push(diagnostic);
        }

        return diagnostics;
    }

    private formatErrorMessage(error: CargoError): string {
        let message = error.message;

        // 添加标签信息
        if (error.spans && error.spans.length > 0) {
            const primarySpan = error.spans.find(s => s.is_primary);
            if (primarySpan && primarySpan.label) {
                message += `\n${primarySpan.label}`;
            }
        }

        // 添加子错误信息
        if (error.children && error.children.length > 0) {
            for (const child of error.children) {
                if (child.level === 'note' || child.level === 'warning') {
                    message += `\n${child.level}: ${child.message}`;
                }
            }
        }

        return message;
    }

    private getSeverity(level: string): vscode.DiagnosticSeverity {
        switch (level) {
            case 'error':
                return vscode.DiagnosticSeverity.Error;
            case 'warning':
                return vscode.DiagnosticSeverity.Warning;
            case 'note':
                return vscode.DiagnosticSeverity.Information;
            default:
                return vscode.DiagnosticSeverity.Error;
        }
    }

    private updateDiagnostics(nuFilePath: string, diagnostics: vscode.Diagnostic[]): void {
        const uri = vscode.Uri.file(nuFilePath);
        this.diagnosticCollection.set(uri, diagnostics);
        this.diagnosticsCache.set(nuFilePath, diagnostics);
    }

    clearDiagnostics(nuFilePath?: string): void {
        if (nuFilePath) {
            const uri = vscode.Uri.file(nuFilePath);
            this.diagnosticCollection.delete(uri);
            this.diagnosticsCache.delete(nuFilePath);
        } else {
            this.diagnosticCollection.clear();
            this.diagnosticsCache.clear();
        }
    }

    getDiagnostics(nuFilePath: string): vscode.Diagnostic[] {
        return this.diagnosticsCache.get(nuFilePath) || [];
    }

    private getMapPath(nuFilePath: string): string {
        const dir = path.dirname(nuFilePath);
        const baseName = path.basename(nuFilePath, '.nu');
        return path.join(dir, `${baseName}.rs.map`);
    }

    dispose(): void {
        this.diagnosticCollection.dispose();
        this.diagnosticsCache.clear();
    }
}