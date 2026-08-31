import { describe, expect, it } from "vitest";

import { filterChannelsBySection } from "./channelSelectors";

const channels = [
  {
    id: "1",
    name: "News",
    streamUrl: "https://example.com/1.m3u8",
    group: "News",
  },
  {
    id: "2",
    name: "Sports",
    streamUrl: "https://example.com/2.m3u8",
    group: "Sports",
  },
];

describe("filterChannelsBySection", () => {
  it("returns all channels for all section", () => {
    expect(
      filterChannelsBySection("all", null, channels, [], []),
    ).toHaveLength(2);
  });

  it("filters by group", () => {
    expect(
      filterChannelsBySection("group", "News", channels, [], []),
    ).toEqual([channels[0]]);
  });
});
