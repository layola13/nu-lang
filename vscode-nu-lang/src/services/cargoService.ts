import * as vscode from 'vscode';
import * as path from 'path';
import { exec } from 'child_process';
import { promisify } from 'util';
import { BinaryManager } from './binaryManager';

const execAsync = promisify(exec);

export interface CargoError {
    message: string;
    level: 'error' | 'warning' | 'note';
    spans: CargoSpan[];
    code?: {
        code: string;
        explanation?: string;
    };
    children?: CargoError[];
}

export interface CargoSpan {
    file_name: string;
    line_start: number;
    line_end: number;
    column_start: number;
    column_end: number;
    is_primary: boolean;
    text: Array<{
        text: string;
        highlight_start: number;
        highlight_end: number;
    }>;
    label?: string;
    suggested_replacement?: string;
}

export interface CargoCheckResult {
    success: boolean;
    errors: CargoError[];
    warnings: CargoError[];
    stdout?: string;
    stderr?: string;
}

export class CargoService {
    constructor(private binaryManager: BinaryManager) {}

    async checkFile(rsFilePath: string): Promise<CargoCheckResult> {
        const cargoPath = this.binaryManager.getCargoPath();
        
        try {
            // 查找 Cargo.toml 所在目录
            const cargoRoot = await this.findCargoRoot(rsFilePath);
            
            if (!cargoRoot) {
                return {
                    success: false,
                    errors: [],
                    warnings: [],
                    stderr: 'Cargo.toml not found'
                };
            }

            // 执行 cargo check，使用 JSON 格式输出
            const command = `${cargoPath} check --message-format=json --color=never`;
            
            const { stdout, stderr } = await execAsync(command, {
                cwd: cargoRoot,
                maxBuffer: 10 * 1024 * 1024 // 10MB
            });

            // 解析 cargo 输出
            return this.parseCargoOutput(stdout, stderr);
        } catch (error: any) {
            // cargo check 返回非零退出码时会抛出错误，但输出仍然有效
            if (error.stdout) {
                return this.parseCargoOutput(error.stdout, error.stderr);
            }
            
            return {
                success: false,
                errors: [],
                warnings: [],
                stderr: error.message
            };
        }
    }

    private parseCargoOutput(stdout: string, stderr: string): CargoCheckResult {
        const errors: CargoError[] = [];
        const warnings: CargoError[] = [];
        let success = true;

        // 按行解析 JSON 输出
        const lines = stdout.split('\n');
        
        for (const line of lines) {
            if (!line.trim()) {
                continue;
            }

            try {
                const message = JSON.parse(line);
                
                // 只处理编译器消息
                if (message.reason !== 'compiler-message') {
                    continue;
                }

                const compilerMessage = message.message;
                
                if (compilerMessage.level === 'error') {
                    errors.push(compilerMessage);
                    success = false;
                } else if (compilerMessage.level === 'warning') {
                    warnings.push(compilerMessage);
                }
            } catch (e) {
                // 忽略非 JSON 行
                continue;
            }
        }

        return {
            success,
            errors,
            warnings,
            stdout,
            stderr
        };
    }

    private async findCargoRoot(startPath: string): Promise<string | null> {
        let currentDir = path.dirname(startPath);
        const root = path.parse(currentDir).root;

        while (currentDir !== root) {
            const cargoTomlPath = path.join(currentDir, 'Cargo.toml');
            
            try {
                const fs = await import('fs');
                if (fs.existsSync(cargoTomlPath)) {
                    return currentDir;
                }
            } catch (error) {
                // 忽略错误，继续向上查找
            }

            currentDir = path.dirname(currentDir);
        }

        return null;
    }

    async checkProject(projectRoot: string): Promise<CargoCheckResult> {
        const cargoPath = this.binaryManager.getCargoPath();
        
        try {
            const command = `${cargoPath} check --message-format=json --color=never`;
            
            const { stdout, stderr } = await execAsync(command, {
                cwd: projectRoot,
                maxBuffer: 10 * 1024 * 1024
            });

            return this.parseCargoOutput(stdout, stderr);
        } catch (error: any) {
            if (error.stdout) {
                return this.parseCargoOutput(error.stdout, error.stderr);
            }
            
            return {
                success: false,
                errors: [],
                warnings: [],
                stderr: error.message
            };
        }
    }

    /**
     * 过滤出与特定文件相关的错误
     */
    filterErrorsForFile(result: CargoCheckResult, rsFilePath: string): CargoCheckResult {
        const normalizedPath = path.normalize(rsFilePath);
        
        const filteredErrors = result.errors.filter(error => 
            error.spans.some(span => 
                path.normalize(span.file_name) === normalizedPath
            )
        );
        
        const filteredWarnings = result.warnings.filter(warning => 
            warning.spans.some(span => 
                path.normalize(span.file_name) === normalizedPath
            )
        );

        return {
            ...result,
            errors: filteredErrors,
            warnings: filteredWarnings
        };
    }
}