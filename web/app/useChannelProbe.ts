import { useCallback, useEffect, useRef, useState } from 'react'

import { lumaApi, toUserMessage } from '@/shared/tauri/api'
import type { ProbeReport, ProbeStatus } from '@/shared/tauri/types'

export interface ProbeSummary {
  playable: number
  unreachable: number
  invalid: number
}

export interface ProbeProgress {
  done: number
  total: number
}

/**
 * State machine for channel availability probing.
 *
 * Probing runs in chunks: each chunk's results are applied (and persisted by
 * the backend) as soon as it finishes, so badges fill in progressively and a
 * mid-run failure keeps all partial results. `progress` drives the
 * "检测中 128/668" toolbar label — without it a multi-minute probe of a
 * 600-channel list looks frozen.
 */
export function useChannelProbe() {
  const [probing, setProbing] = useState(false)
  const [probeStatusById, setProbeStatusById] = useState<Record<string, ProbeStatus>>({})
  const [probeSummary, setProbeSummary] = useState<ProbeSummary | null>(null)
  const [progress, setProgress] = useState<ProbeProgress | null>(null)
  const runningRef = useRef(false)
  // Latest progress readable from the catch handler without adding it to
  // the callback deps.
  const progressRef = useRef<ProbeProgress | null>(null)
  progressRef.current = progress

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
    // Chunk summaries accumulate in runProbeChunked; the hook-level summary
    // reflects the latest chunk for immediate UI feedback.
    setProbeSummary({
      playable: report.playable,
      unreachable: report.unreachable,
      invalid: report.invalid
    })
  }, [])

  const runProbeChunked = useCallback(
    async (channelIds: string[], chunkSize = 64): Promise<string> => {
      if (runningRef.current) {
        return '检测正在进行中'
      }
      if (channelIds.length === 0) {
        return '没有可检测的频道'
      }
      runningRef.current = true
      setProbing(true)
      setProgress({ done: 0, total: channelIds.length })

      const totals = { playable: 0, unreachable: 0, invalid: 0 }
      try {
        for (let start = 0; start < channelIds.length; start += chunkSize) {
          // Sequential on purpose: chunks apply their results progressively
          // and the backend already probes each chunk concurrently.
          // eslint-disable-next-line no-await-in-loop
          const report = await lumaApi.probeChannels(channelIds.slice(start, start + chunkSize))
          applyReport(report)
          totals.playable += report.playable
          totals.unreachable += report.unreachable
          totals.invalid += report.invalid
          setProgress({
            done: Math.min(start + chunkSize, channelIds.length),
            total: channelIds.length
          })
        }
        setProbeSummary(totals)
        return `检测完成：可用 ${totals.playable}，不可达 ${totals.unreachable}，无效 ${totals.invalid}`
      } catch (err) {
        // Partial results are already applied and persisted.
        return `检测中断（已检测 ${progressRef.current?.done ?? 0}/${channelIds.length}）：${toUserMessage(err)}`
      } finally {
        runningRef.current = false
        setProbing(false)
        setProgress(null)
      }
    },
    [applyReport]
  )

  return { probing, probeStatusById, probeSummary, progress, runProbeChunked }
}
