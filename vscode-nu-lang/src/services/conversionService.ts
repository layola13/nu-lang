import * as vscode from 'vscode';
import * as path from 'path';
import { exec } from 'child_process';
import { promisify } from 'util';
import { BinaryManager } from './binaryManager';

const execAsync = promisify(exec);

export interface ConversionResult {
    success: boolean;
    outputPath?: string;
    mapPath?: string;
    error?: string;
    stdout?: string;
    stderr?: string;
}

export class ConversionService {
    constructor(private binaryManager: BinaryManager) {}

    async convertFile(nuFilePath: string): Promise<ConversionResult> {
        const nu2rustPath = this.binaryManager.getNu2rustPath();
        
        if (!nu2rustPath) {
            return {
                success: false,
                error: 'nu2rust binary not found'
            };
        }

        try {
            // 构建输出路径
            const dir = path.dirname(nuFilePath);
            const baseName = path.basename(nuFilePath, '.nu');
            const outputPath = path.join(dir, `${baseName}.rs`);
            const mapPath = path.join(dir, `${baseName}.rs.map`);

            // 执行 nu2rust 命令
            // 格式: nu2rust <input.nu> -o <output.rs> --sourcemap -f
            const command = `"${nu2rustPath}" "${nuFilePath}" -o "${outputPath}" --sourcemap -f`;
            
            const { stdout, stderr } = await execAsync(command, {
                cwd: path.dirname(nuFilePath),
                maxBuffer: 10 * 1024 * 1024 // 10MB
            });

            // 格式化生成的 Rust 代码
            try {
                await execAsync(`rustfmt "${outputPath}"`, {
                    cwd: path.dirname(nuFilePath)
                });
            } catch (fmtError: any) {
                // rustfmt 失败不影响整体流程，只记录警告
                console.warn(`rustfmt failed for ${outputPath}:`, fmtError.message);
            }

            return {
                success: true,
                outputPath,
                mapPath,
                stdout,
                stderr
            };
        } catch (error: any) {
            return {
                success: false,
                error: error.message,
                stdout: error.stdout,
                stderr: error.stderr
            };
        }
    }

    async convertFileWithProgress(
        nuFilePath: string,
        progress: vscode.Progress<{ message?: string; increment?: number }>
    ): Promise<ConversionResult> {
        progress.report({ message: 'Converting Nu to Rust...', increment: 0 });
        
        const result = await this.convertFile(nuFilePath);
        
        if (result.success) {
            progress.report({ message: 'Conversion completed', increment: 100 });
        } else {
            progress.report({ message: 'Conversion failed', increment: 100 });
        }
        
        return result;
    }

    async batchConvert(nuFilePaths: string[]): Promise<ConversionResult[]> {
        const results: ConversionResult[] = [];
        
        for (const filePath of nuFilePaths) {
            const result = await this.convertFile(filePath);
            results.push(result);
        }
        
        return results;
    }

    getOutputPath(nuFilePath: string): string {
        const dir = path.dirname(nuFilePath);
        const baseName = path.basename(nuFilePath, '.nu');
        return path.join(dir, `${baseName}.rs`);
    }

    getMapPath(nuFilePath: string): string {
        const dir = path.dirname(nuFilePath);
        const baseName = path.basename(nuFilePath, '.nu');
        return path.join(dir, `${baseName}.rs.map`);
    }
}