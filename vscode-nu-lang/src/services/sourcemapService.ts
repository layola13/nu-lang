import * as fs from 'fs';
import * as path from 'path';

export interface SourceMapMapping {
    nuLine: number;
    nuColumn: number;
    rsLine: number;
    rsColumn: number;
    name?: string;
}

export interface SourceMap {
    version: number;
    file: string;
    sourceRoot?: string;
    sources: string[];
    sourcesContent?: string[];
    names?: string[];
    mappings: SourceMapMapping[];
}

export class SourcemapService {
    private cache: Map<string, SourceMap> = new Map();

    async loadSourceMap(mapFilePath: string): Promise<SourceMap | null> {
        // 检查缓存
        if (this.cache.has(mapFilePath)) {
            return this.cache.get(mapFilePath)!;
        }

        try {
            // 读取 .map 文件
            const content = await fs.promises.readFile(mapFilePath, 'utf-8');
            const mapData = JSON.parse(content);

            // 解析 SourceMap
            const sourceMap = this.parseSourceMap(mapData);
            
            // 缓存结果
            this.cache.set(mapFilePath, sourceMap);
            
            return sourceMap;
        } catch (error) {
            console.error('Failed to load source map:', error);
            return null;
        }
    }

    private parseSourceMap(data: any): SourceMap {
        // 简化版 SourceMap 解析
        // 假设 nu2rust 生成的格式是简单的 JSON 数组
        const mappings: SourceMapMapping[] = [];

        if (data.mappings && Array.isArray(data.mappings)) {
            for (const mapping of data.mappings) {
                mappings.push({
                    nuLine: mapping.nu_line || mapping.nuLine || 0,
                    nuColumn: mapping.nu_column || mapping.nuColumn || 0,
                    rsLine: mapping.rs_line || mapping.rsLine || 0,
                    rsColumn: mapping.rs_column || mapping.rsColumn || 0,
                    name: mapping.name
                });
            }
        }

        return {
            version: data.version || 3,
            file: data.file || '',
            sourceRoot: data.sourceRoot,
            sources: data.sources || [],
            sourcesContent: data.sourcesContent,
            names: data.names,
            mappings
        };
    }

    /**
     * 从 Rust 行号映射到 Nu 行号
     */
    mapRustToNu(mapFilePath: string, rsLine: number, rsColumn: number = 0): SourceMapMapping | null {
        const sourceMap = this.cache.get(mapFilePath);
        if (!sourceMap) {
            return null;
        }

        // 查找最接近的映射
        let bestMatch: SourceMapMapping | null = null;
        let minDistance = Infinity;

        for (const mapping of sourceMap.mappings) {
            if (mapping.rsLine === rsLine) {
                const distance = Math.abs(mapping.rsColumn - rsColumn);
                if (distance < minDistance) {
                    minDistance = distance;
                    bestMatch = mapping;
                }
            } else if (mapping.rsLine < rsLine && rsLine - mapping.rsLine < minDistance) {
                // 如果没有精确匹配，使用最近的前一行
                minDistance = rsLine - mapping.rsLine;
                bestMatch = mapping;
            }
        }

        return bestMatch;
    }

    /**
     * 从 Nu 行号映射到 Rust 行号
     */
    mapNuToRust(mapFilePath: string, nuLine: number, nuColumn: number = 0): SourceMapMapping | null {
        const sourceMap = this.cache.get(mapFilePath);
        if (!sourceMap) {
            return null;
        }

        // 查找最接近的映射
        let bestMatch: SourceMapMapping | null = null;
        let minDistance = Infinity;

        for (const mapping of sourceMap.mappings) {
            if (mapping.nuLine === nuLine) {
                const distance = Math.abs(mapping.nuColumn - nuColumn);
                if (distance < minDistance) {
                    minDistance = distance;
                    bestMatch = mapping;
                }
            } else if (mapping.nuLine < nuLine && nuLine - mapping.nuLine < minDistance) {
                minDistance = nuLine - mapping.nuLine;
                bestMatch = mapping;
            }
        }

        return bestMatch;
    }

    /**
     * 清除缓存
     */
    clearCache(mapFilePath?: string): void {
        if (mapFilePath) {
            this.cache.delete(mapFilePath);
        } else {
            this.cache.clear();
        }
    }

    /**
     * 预加载 SourceMap
     */
    async preloadSourceMap(nuFilePath: string): Promise<void> {
        const mapPath = this.getMapPath(nuFilePath);
        if (fs.existsSync(mapPath)) {
            await this.loadSourceMap(mapPath);
        }
    }

    private getMapPath(nuFilePath: string): string {
        const dir = path.dirname(nuFilePath);
        const baseName = path.basename(nuFilePath, '.nu');
        return path.join(dir, `${baseName}.rs.map`);
    }

    /**
     * 检查 SourceMap 是否存在
     */
    hasSourceMap(nuFilePath: string): boolean {
        const mapPath = this.getMapPath(nuFilePath);
        return fs.existsSync(mapPath);
    }
}