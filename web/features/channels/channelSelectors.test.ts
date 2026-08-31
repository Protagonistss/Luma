import { describe, expect, it } from "vitest";

import {
  buildChannelVirtualRows,
  filterChannelsBySection,
  getChannelGridColumnCount,
  groupChannelsByShelf,
} from "./channelSelectors";

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

  it("groups channels into shelves by category", () => {
    expect(groupChannelsByShelf(channels)).toEqual([
      { title: "News", channels: [channels[0]] },
      { title: "Sports", channels: [channels[1]] },
    ]);
  });

  it("calculates grid columns from container width", () => {
    expect(getChannelGridColumnCount(1200)).toBe(6);
    expect(getChannelGridColumnCount(400)).toBe(1);
  });

  it("builds virtual rows with shelf headers", () => {
    expect(buildChannelVirtualRows(channels, 1, true)).toEqual([
      { kind: "shelf-header", title: "News", count: 1, key: "header:News" },
      { kind: "channel-row", channels: [channels[0]], key: "row:News:0" },
      { kind: "shelf-header", title: "Sports", count: 1, key: "header:Sports" },
      { kind: "channel-row", channels: [channels[1]], key: "row:Sports:0" },
    ]);
  });

  it("builds flat virtual rows without shelf headers", () => {
    expect(buildChannelVirtualRows(channels, 2, false)).toEqual([
      { kind: "channel-row", channels, key: "row:0" },
    ]);
  });
});
