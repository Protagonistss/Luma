import { useCallback, useEffect, useRef, useState } from 'react'

import { lumaApi, toUserMessage } from '@/shared/tauri/api'
import type { ProbeReport, ProbeStatus } from '@/shared/tauri/types'

export interface ProbeSummary {
  playable: number
  unreachable: number
  invalid: number
}

/**
 * State machine for channel availability probing. `runProbe` always resolves
 * with a toast-ready message: a summary line on success or an error message.
 * On mount the persisted probe cache (written by the backend after every
 * probe) is loaded so cards keep their live/offline badges across restarts.
 */
export function useChannelProbe() {
  const [probing, setProbing] = useState(false)
  const [probeStatusById, setProbeStatusById] = useState<Record<string, ProbeStatus>>({})
  const [probeSummary, setProbeSummary] = useState<ProbeSummary | null>(null)
  const runningRef = useRef(false)

  useEffect(() => {
    lumaApi
      .getProbeStatus()
      .then((cached) => setProbeStatusById(cached))
      .catch(() => undefined)
  }, [])

  const applyReport = useCallback((report: ProbeReport) => {
    setProbeStatusById((previous) => {
      const next = { ...previous }
      for (const result of report.results) {
        next[result.channelId] = result.status
      }
      return next
    })
    setProbeSummary({
      playable: report.playable,
      unreachable: report.unreachable,
      invalid: report.invalid
    })
  }, [])

  const runProbe = useCallback(
    async (channelIds?: string[]): Promise<string> => {
      if (runningRef.current) {
        return '检测正在进行中'
      }
      runningRef.current = true
      setProbing(true)
      try {
        const report = await lumaApi.probeChannels(channelIds)
        applyReport(report)
        return `检测完成：可用 ${report.playable}，不可达 ${report.unreachable}，无效 ${report.invalid}`
      } catch (err) {
        return toUserMessage(err)
      } finally {
        runningRef.current = false
        setProbing(false)
      }
    },
    [applyReport]
  )

  return { probing, probeStatusById, probeSummary, runProbe }
}
