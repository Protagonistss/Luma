import ithinku from '@ithinku/oxfmt-config'
import { defineConfig } from 'oxfmt'

export default defineConfig({
  ...ithinku,
  ignorePatterns: [
    ...(ithinku.ignorePatterns ?? []),
    'native/gen',
    'native/target-test',
    // Tauri 插件自动生成的权限文档/schema，不参与格式化
    'native/plugins/player/permissions/**'
  ]
})
