import ithinku from '@ithinku/oxlint-config'
import { defineConfig } from 'oxlint'

export default defineConfig({
  extends: [ithinku],
  // 项目内补充：Tauri 生成的 Android 工程不参与前端 lint
  ignorePatterns: [...(ithinku.ignorePatterns ?? []), 'native/gen', 'native/target-test']
})
