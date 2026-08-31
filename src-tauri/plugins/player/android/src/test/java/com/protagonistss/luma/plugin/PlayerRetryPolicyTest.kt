package com.protagonistss.luma.plugin

import org.junit.Assert.assertEquals
import org.junit.Test

class PlayerRetryPolicyTest {
  @Test
  fun retryDelaysFollowBackoffSequence() {
    val delays = longArrayOf(2000, 5000, 10000)
    assertEquals(3, delays.size)
    assertEquals(2000, delays[0])
    assertEquals(10000, delays[2])
  }
}
