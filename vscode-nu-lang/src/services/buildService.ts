import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs/promises';
import { exec } from 'child_process';
import { promisify } from 'util';
import { BinaryManager } from './binaryManager';

const execAsync = promisify(exec);

export interface BuildResult {
    success: boolean;
    binaryPath?: string;
    error?: string;
    stdout?: string;
    stderr?: string;
}

export class BuildService {
    constructor(private binaryManager: BinaryManager) {}

    /**
     * 构建二进制文件
     * @param nuFilePath Nu 源文件路径
     * @param releaseMode 是否使用 release 模式
     */
    async buildBinary(nuFilePath: string, releaseMode: boolean = false): Promise<BuildResult> {
        try {
            // 步骤 1: 确保 .rs 文件存在
            const rsFilePath = await this.ensureRustFile(nuFilePath);
            
            // 步骤 2: 检测项目类型并编译
            const isCargoProject = await this.isInCargoProject(rsFilePath);
            
            if (isCargoProject) {
                return await this.buildWithCargo(rsFilePath, releaseMode);
            } else {
                return await this.buildWithRustc(rsFilePath);
            }
        } catch (error: any) {
            return {
                success: false,
                error: error.message || 'Build failed'
            };
        }
    }

    /**
     * 构建当前文件的二进制
     */
    async buildCurrentFile(releaseMode: boolean = false): Promise<void> {
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

        // 保存文件
        if (document.isDirty) {
            await document.save();
        }

        // 显示进度提示
        await vscode.window.withProgress(
            {
                location: vscode.ProgressLocation.Notification,
                title: 'Building binary...',
                cancellable: false
            },
            async (progress) => {
                try {
                    progress.report({ message: 'Preparing...', increment: 10 });
                    
                    const result = await this.buildBinary(document.uri.fsPath, releaseMode);
                    
                    if (result.success && result.binaryPath) {
                        progress.report({ message: 'Build completed', increment: 100 });
                        
                        vscode.window.showInformationMessage(
                            `Binary built successfully: ${result.binaryPath}`,
                            'Open Folder'
                        ).then(selection => {
                            if (selection === 'Open Folder') {
                                vscode.commands.executeCommand(
                                    'revealFileInOS',
                                    vscode.Uri.file(result.binaryPath!)
                                );
                            }
                        });
                    } else {
                        throw new Error(result.error || 'Build failed');
                    }
                } catch (error: any) {
                    vscode.window.showErrorMessage(`Build failed: ${error.message}`);
                }
            }
        );
    }

    /**
     * 确保 Rust 文件存在，如果不存在则先编译
     */
    private async ensureRustFile(nuFilePath: string): Promise<string> {
        const dir = path.dirname(nuFilePath);
        const baseName = path.basename(nuFilePath, '.nu');
        const rsFilePath = path.join(dir, `${baseName}.rs`);
        
        try {
            await fs.access(rsFilePath);
            return rsFilePath;
        } catch {
            // .rs 文件不存在，需要先编译
            const nu2rustPath = this.binaryManager.getNu2rustPath();
            
            if (!nu2rustPath) {
                throw new Error('nu2rust binary not found. Please configure nu-lang.nu2rustPath in settings.');
            }
            
            const command = `"${nu2rustPath}" "${nuFilePath}" -o "${rsFilePath}" -f`;
            
            await execAsync(command, {
                cwd: dir,
                maxBuffer: 10 * 1024 * 1024
            });
            
            return rsFilePath;
        }
    }

    /**
     * 检测是否在 Cargo 项目中，且是项目的源代码文件
     */
    private async isInCargoProject(rsFilePath: string): Promise<boolean> {
        const fileDir = path.dirname(rsFilePath);
        let currentDir = fileDir;
        const root = path.parse(currentDir).root;
        
        // 向上查找 Cargo.toml
        while (currentDir !== root) {
            const cargoTomlPath = path.join(currentDir, 'Cargo.toml');
            
            try {
                await fs.access(cargoTomlPath);
                
                // 找到了 Cargo.toml，检查文件是否在项目的 src/ 目录中
                const projectRoot = currentDir;
                const srcDir = path.join(projectRoot, 'src');
                const relativePath = path.relative(projectRoot, rsFilePath);
                
                // 如果文件在 src/ 或 src/bin/ 目录中，才认为是 Cargo 项目的源文件
                if (relativePath.startsWith('src' + path.sep) || relativePath.startsWith('src/')) {
                    return true;
                }
                
                // 否则认为是独立文件，使用 rustc 编译
                return false;
            } catch {
                // 继续向上查找
            }
            
            currentDir = path.dirname(currentDir);
        }
        
        return false;
    }

    /**
     * 使用 Cargo 构建
     */
    private async buildWithCargo(rsFilePath: string, releaseMode: boolean): Promise<BuildResult> {
        const cargoPath = this.binaryManager.getCargoPath();
        const projectRoot = await this.findCargoRoot(rsFilePath);
        
        if (!projectRoot) {
            return {
                success: false,
                error: 'Cargo.toml not found'
            };
        }
        
        try {
            const releaseFlag = releaseMode ? '--release' : '';
            const command = `${cargoPath} build ${releaseFlag}`.trim();
            
            const { stdout, stderr } = await execAsync(command, {
                cwd: projectRoot,
                maxBuffer: 10 * 1024 * 1024
            });
            
            // 确定二进制路径
            const binaryPath = await this.findCargoBinaryPath(projectRoot, releaseMode);
            
            return {
                success: true,
                binaryPath,
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

    /**
     * 使用 rustc 构建独立文件
     */
    private async buildWithRustc(rsFilePath: string): Promise<BuildResult> {
        const rustcPath = this.binaryManager.getRustcPath();
        
        try {
            // 检查 rustc 是否可用
            await execAsync(`${rustcPath} --version`).catch(() => {
                throw new Error('rustc not found in PATH. Please install Rust or configure nu-lang.rustcPath');
            });
            
            const dir = path.dirname(rsFilePath);
            const baseName = path.basename(rsFilePath, '.rs');
            const isWindows = process.platform === 'win32';
            const binaryPath = path.join(dir, baseName + (isWindows ? '.exe' : ''));
            
            // rustc 命令格式: rustc input.rs -o output
            const command = `${rustcPath} "${rsFilePath}" -o "${binaryPath}"`;
            
            const { stdout, stderr } = await execAsync(command, {
                cwd: dir,
                maxBuffer: 10 * 1024 * 1024
            });
            
            // 验证二进制文件是否生成
            try {
                await fs.access(binaryPath);
            } catch {
                throw new Error(`Binary file not created at: ${binaryPath}`);
            }
            
            return {
                success: true,
                binaryPath,
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

    /**
     * 查找 Cargo 项目根目录
     */
    private async findCargoRoot(startPath: string): Promise<string | null> {
        let currentDir = path.dirname(startPath);
        const root = path.parse(currentDir).root;
        
        while (currentDir !== root) {
            const cargoTomlPath = path.join(currentDir, 'Cargo.toml');
            
            try {
                await fs.access(cargoTomlPath);
                return currentDir;
            } catch {
                // 继续向上查找
            }
            
            currentDir = path.dirname(currentDir);
        }
        
        return null;
    }

    /**
     * 查找 Cargo 构建的二进制路径
     */
    private async findCargoBinaryPath(projectRoot: string, releaseMode: boolean): Promise<string> {
        const buildDir = releaseMode ? 'release' : 'debug';
        const targetDir = path.join(projectRoot, 'target', buildDir);
        
        try {
            // 读取 Cargo.toml 获取项目名
            const cargoTomlPath = path.join(projectRoot, 'Cargo.toml');
            const cargoTomlContent = await fs.readFile(cargoTomlPath, 'utf-8');
            
            // 简单解析项目名
            const nameMatch = cargoTomlContent.match(/name\s*=\s*"([^"]+)"/);
            const projectName = nameMatch ? nameMatch[1] : 'unknown';
            
            const isWindows = process.platform === 'win32';
            const binaryName = projectName + (isWindows ? '.exe' : '');
            const binaryPath = path.join(targetDir, binaryName);
            
            // 验证二进制文件是否存在
            await fs.access(binaryPath);
            
            return binaryPath;
        } catch (error) {
            // 如果无法确定具体文件，返回目录
            return targetDir;
        }
    }

    /**
     * 构建并运行
     */
    async buildAndRun(nuFilePath: string, args: string[] = []): Promise<void> {
        const result = await this.buildBinary(nuFilePath, false);
        
        if (!result.success || !result.binaryPath) {
            vscode.window.showErrorMessage(`Build failed: ${result.error}`);
            return;
        }
        
        // 在终端中运行
        const terminal = vscode.window.createTerminal('Nu Binary');
        const argsStr = args.map(arg => `"${arg}"`).join(' ');
        terminal.sendText(`"${result.binaryPath}" ${argsStr}`);
        terminal.show();
    }

    /**
     * 构建带调试信息的二进制文件
     * @param nuFilePath Nu 源文件路径
     */
    async buildForDebug(nuFilePath: string): Promise<BuildResult> {
        try {
            // 步骤 1: 确保 .rs 文件存在
            const rsFilePath = await this.ensureRustFile(nuFilePath);
            
            // 步骤 2: 检测项目类型并编译
            const isCargoProject = await this.isInCargoProject(rsFilePath);
            
            if (isCargoProject) {
                return await this.buildWithCargoDebug(rsFilePath);
            } else {
                return await this.buildWithRustcDebug(rsFilePath);
            }
        } catch (error: any) {
            return {
                success: false,
                error: error.message || 'Debug build failed'
            };
        }
    }

    /**
     * 使用 Cargo 构建调试版本（带调试信息）
     */
    private async buildWithCargoDebug(rsFilePath: string): Promise<BuildResult> {
        const cargoPath = this.binaryManager.getCargoPath();
        const projectRoot = await this.findCargoRoot(rsFilePath);
        
        if (!projectRoot) {
            return {
                success: false,
                error: 'Cargo.toml not found'
            };
        }
        
        try {
            // 使用 debug 模式构建（默认带调试信息）
            const command = `${cargoPath} build`;
            
            const { stdout, stderr } = await execAsync(command, {
                cwd: projectRoot,
                maxBuffer: 10 * 1024 * 1024
            });
            
            // 确定二进制路径
            const binaryPath = await this.findCargoBinaryPath(projectRoot, false);
            
            return {
                success: true,
                binaryPath,
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

    /**
     * 使用 rustc 构建带调试信息的独立文件
     */
    private async buildWithRustcDebug(rsFilePath: string): Promise<BuildResult> {
        const rustcPath = this.binaryManager.getRustcPath();
        
        try {
            // 检查 rustc 是否可用
            await execAsync(`${rustcPath} --version`).catch(() => {
                throw new Error('rustc not found in PATH. Please install Rust or configure nu-lang.rustcPath');
            });
            
            const dir = path.dirname(rsFilePath);
            const baseName = path.basename(rsFilePath, '.rs');
            const isWindows = process.platform === 'win32';
            const binaryPath = path.join(dir, baseName + '_debug' + (isWindows ? '.exe' : ''));
            
            // rustc 命令格式: rustc -g input.rs -o output
            // -g 选项启用调试信息
            // -C debuginfo=2 提供完整的调试信息（包括变量信息）
            const command = `${rustcPath} -g -C debuginfo=2 "${rsFilePath}" -o "${binaryPath}"`;
            
            const { stdout, stderr } = await execAsync(command, {
                cwd: dir,
                maxBuffer: 10 * 1024 * 1024
            });
            
            // 验证二进制文件是否生成
            try {
                await fs.access(binaryPath);
            } catch {
                throw new Error(`Binary file not created at: ${binaryPath}`);
            }
            
            return {
                success: true,
                binaryPath,
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
}