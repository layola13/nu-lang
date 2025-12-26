import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs/promises';
import { exec } from 'child_process';
import { promisify } from 'util';
import { BinaryManager } from './binaryManager';

const execAsync = promisify(exec);

export interface FormatResult {
    success: boolean;
    error?: string;
    formatted?: string;
}

export class FormatService {
    constructor(private binaryManager: BinaryManager) {}

    /**
     * 格式化 Nu 文件
     * 实现方案: Nu → Rust → rustfmt → Rust → Nu 循环
     */
    async formatNuFile(nuFilePath: string): Promise<FormatResult> {
        const nu2rustPath = this.binaryManager.getNu2rustPath();
        
        if (!nu2rustPath) {
            return {
                success: false,
                error: 'nu2rust binary not found. Please configure nu-lang.nu2rustPath in settings.'
            };
        }

        // 创建临时文件路径
        const dir = path.dirname(nuFilePath);
        const baseName = path.basename(nuFilePath, '.nu');
        const tempRsPath = path.join(dir, `.${baseName}.temp.rs`);
        const tempNuPath = path.join(dir, `.${baseName}.temp.nu`);
        
        try {
            // 步骤 1: Nu → Rust
            await this.convertNuToRust(nuFilePath, tempRsPath, nu2rustPath);
            
            // 步骤 2: 格式化 Rust 代码
            await this.formatRustFile(tempRsPath);
            
            // 步骤 3: Rust → Nu
            const formattedNu = await this.convertRustToNu(tempRsPath, tempNuPath);
            
            return {
                success: true,
                formatted: formattedNu
            };
        } catch (error: any) {
            return {
                success: false,
                error: error.message || 'Format failed'
            };
        } finally {
            // 清理临时文件
            await this.cleanupTempFiles([tempRsPath, tempNuPath, `${tempRsPath}.map`]);
        }
    }

    /**
     * 格式化当前编辑器中的 Nu 文件
     */
    async formatCurrentFile(): Promise<void> {
        const editor = vscode.window.activeTextEditor;
        
        if (!editor) {
            vscode.window.showWarningMessage('No active editor');
            return;
        }

        const document = editor.document;
        
        // 检查文件类型
        if (document.languageId !== 'nu' && !document.fileName.endsWith('.nu')) {
            vscode.window.showWarningMessage('Current file is not a Nu file');
            return;
        }

        // 保存文件（如果有未保存的更改）
        if (document.isDirty) {
            await document.save();
        }

        // 显示进度提示
        await vscode.window.withProgress(
            {
                location: vscode.ProgressLocation.Notification,
                title: 'Formatting Nu code...',
                cancellable: false
            },
            async (progress) => {
                try {
                    progress.report({ message: 'Converting to Rust...', increment: 25 });
                    
                    const result = await this.formatNuFile(document.uri.fsPath);
                    
                    if (result.success && result.formatted) {
                        progress.report({ message: 'Applying format...', increment: 75 });
                        
                        // 替换编辑器内容
                        await this.replaceEditorContent(editor, result.formatted);
                        
                        vscode.window.showInformationMessage('Nu file formatted successfully');
                    } else {
                        throw new Error(result.error || 'Format failed');
                    }
                } catch (error: any) {
                    vscode.window.showErrorMessage(`Format failed: ${error.message}`);
                }
            }
        );
    }

    /**
     * Nu → Rust 转换
     */
    private async convertNuToRust(
        nuFilePath: string,
        outputPath: string,
        nu2rustPath: string
    ): Promise<void> {
        const command = `"${nu2rustPath}" "${nuFilePath}" -o "${outputPath}" -f`;
        
        try {
            await execAsync(command, {
                cwd: path.dirname(nuFilePath),
                maxBuffer: 10 * 1024 * 1024
            });
        } catch (error: any) {
            throw new Error(`Nu to Rust conversion failed: ${error.message}`);
        }
    }

    /**
     * 格式化 Rust 文件
     */
    private async formatRustFile(rsFilePath: string): Promise<void> {
        try {
            await execAsync(`rustfmt "${rsFilePath}"`, {
                cwd: path.dirname(rsFilePath)
            });
        } catch (error: any) {
            throw new Error(`rustfmt failed: ${error.message}`);
        }
    }

    /**
     * Rust → Nu 转换
     */
    private async convertRustToNu(rsFilePath: string, outputPath: string): Promise<string> {
        // 从 BinaryManager 获取 rust2nu 路径
        const rust2nuPath = this.binaryManager.getRust2nuPath();
        
        if (!rust2nuPath) {
            throw new Error('rust2nu binary not found. Please install rust2nu or configure nu-lang.rust2nuPath in settings.');
        }
        
        try {
            const command = `"${rust2nuPath}" "${rsFilePath}" -o "${outputPath}"`;
            
            await execAsync(command, {
                cwd: path.dirname(rsFilePath),
                maxBuffer: 10 * 1024 * 1024
            });
            
            // 读取转换后的内容
            const formatted = await fs.readFile(outputPath, 'utf-8');
            return formatted;
        } catch (error: any) {
            throw new Error(`Rust to Nu conversion failed: ${error.message}`);
        }
    }

    /**
     * 替换编辑器内容
     */
    private async replaceEditorContent(
        editor: vscode.TextEditor,
        newContent: string
    ): Promise<void> {
        const document = editor.document;
        const fullRange = new vscode.Range(
            document.positionAt(0),
            document.positionAt(document.getText().length)
        );
        
        await editor.edit(editBuilder => {
            editBuilder.replace(fullRange, newContent);
        });
    }

    /**
     * 清理临时文件
     */
    private async cleanupTempFiles(filePaths: string[]): Promise<void> {
        for (const filePath of filePaths) {
            try {
                await fs.unlink(filePath);
            } catch (error) {
                // 忽略清理错误
            }
        }
    }
}