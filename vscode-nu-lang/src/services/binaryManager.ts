import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
import { exec } from 'child_process';
import { promisify } from 'util';

const execAsync = promisify(exec);

export class BinaryManager {
    private nu2rustPath: string | null = null;
    private rust2nuPath: string | null = null;
    private cargoPath: string = 'cargo';
    private rustcPath: string = 'rustc';

    constructor(private context: vscode.ExtensionContext) {}

    async initialize(): Promise<void> {
        // 从配置读取路径
        const config = vscode.workspace.getConfiguration('nu-lang');
        
        // 检测 nu2rust
        const configuredNu2rust = config.get<string>('nu2rustPath');
        if (configuredNu2rust && fs.existsSync(configuredNu2rust)) {
            this.nu2rustPath = configuredNu2rust;
        } else {
            await this.detectNu2rust();
        }
        
        // 检测 rust2nu
        const configuredRust2nu = config.get<string>('rust2nuPath');
        if (configuredRust2nu && fs.existsSync(configuredRust2nu)) {
            this.rust2nuPath = configuredRust2nu;
        } else {
            await this.detectRust2nu();
        }
        
        // 检测 cargo 和 rustc
        this.cargoPath = config.get<string>('cargoPath', 'cargo');
        this.rustcPath = config.get<string>('rustcPath', 'rustc');
    }

    private async detectNu2rust(): Promise<void> {
        const searchPaths = [
            // 当前项目的 target/release
            path.join(vscode.workspace.workspaceFolders?.[0]?.uri.fsPath || '', 'target', 'release', 'nu2rust'),
            // 当前项目的 target/debug
            path.join(vscode.workspace.workspaceFolders?.[0]?.uri.fsPath || '', 'target', 'debug', 'nu2rust'),
            // 系统 PATH 中的 nu2rust
            'nu2rust',
        ];

        // Windows 平台添加 .exe 后缀
        const isWindows = process.platform === 'win32';
        if (isWindows) {
            searchPaths[0] += '.exe';
            searchPaths[1] += '.exe';
        }

        // 尝试每个路径
        for (const testPath of searchPaths) {
            try {
                if (testPath === 'nu2rust' || (isWindows && testPath === 'nu2rust.exe')) {
                    // 测试 PATH 中的命令
                    await execAsync(`${testPath} --version`);
                    this.nu2rustPath = testPath;
                    return;
                } else if (fs.existsSync(testPath)) {
                    // 测试文件系统路径
                    this.nu2rustPath = testPath;
                    return;
                }
            } catch (error) {
                // 继续尝试下一个路径
                continue;
            }
        }

        // 未找到二进制
        vscode.window.showWarningMessage(
            'nu2rust binary not found. Please set nu-lang.nu2rustPath in settings or build the project.'
        );
    }

    getNu2rustPath(): string | null {
        return this.nu2rustPath;
    }

    getRust2nuPath(): string | null {
        return this.rust2nuPath;
    }

    getCargoPath(): string {
        return this.cargoPath;
    }

    getRustcPath(): string {
        return this.rustcPath;
    }

    isNu2rustAvailable(): boolean {
        return this.nu2rustPath !== null;
    }

    isRust2nuAvailable(): boolean {
        return this.rust2nuPath !== null;
    }

    /**
     * 检测 rust2nu 二进制
     */
    private async detectRust2nu(): Promise<void> {
        const searchPaths = [
            '/usr/local/bin/rust2nu',
            '/usr/bin/rust2nu',
            path.join(process.env.HOME || '', '.cargo', 'bin', 'rust2nu'),
            'rust2nu' // PATH 中的命令
        ];

        const isWindows = process.platform === 'win32';
        if (isWindows) {
            searchPaths[0] += '.exe';
            searchPaths[1] += '.exe';
            searchPaths[2] += '.exe';
        }

        for (const testPath of searchPaths) {
            try {
                if (testPath === 'rust2nu' || (isWindows && testPath === 'rust2nu.exe')) {
                    // 测试 PATH 中的命令
                    await execAsync(`${testPath} --version`);
                    this.rust2nuPath = testPath;
                    return;
                } else if (fs.existsSync(testPath)) {
                    // 测试文件系统路径
                    this.rust2nuPath = testPath;
                    return;
                }
            } catch (error) {
                continue;
            }
        }

        // rust2nu 不是必须的（只有格式化功能需要）
        console.warn('rust2nu binary not found. Format feature will be unavailable.');
    }

    async verifyBinaries(): Promise<{ nu2rust: boolean; rust2nu: boolean; cargo: boolean; rustc: boolean }> {
        const result = { nu2rust: false, rust2nu: false, cargo: false, rustc: false };

        // 验证 nu2rust
        if (this.nu2rustPath) {
            try {
                await execAsync(`"${this.nu2rustPath}" --version`);
                result.nu2rust = true;
            } catch (error) {
                console.error('nu2rust verification failed:', error);
            }
        }

        // 验证 rust2nu
        if (this.rust2nuPath) {
            try {
                await execAsync(`"${this.rust2nuPath}" --version`);
                result.rust2nu = true;
            } catch (error) {
                console.error('rust2nu verification failed:', error);
            }
        }

        // 验证 cargo
        try {
            await execAsync(`${this.cargoPath} --version`);
            result.cargo = true;
        } catch (error) {
            console.error('cargo verification failed:', error);
        }

        // 验证 rustc
        try {
            await execAsync(`${this.rustcPath} --version`);
            result.rustc = true;
        } catch (error) {
            console.error('rustc verification failed:', error);
        }

        return result;
    }
}