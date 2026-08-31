import type { Channel } from "@/shared/tauri/types";

export type SidebarSection = "all" | "favorites" | "recent" | "group";

export function filterChannelsBySection(
  section: SidebarSection,
  groupName: string | null,
  allChannels: Channel[],
  favorites: Channel[],
  recent: Channel[],
): Channel[] {
  switch (section) {
    case "favorites":
      return favorites;
    case "recent":
      return recent;
    case "group":
      return groupName
        ? allChannels.filter((channel) => channel.group === groupName)
        : allChannels;
    case "all":
    default:
      return allChannels;
  }
}

export function buildSidebarItems(
  groups: { name: string; channelCount: number }[],
) {
  return [
    { id: "all" as const, label: "全部" },
    { id: "favorites" as const, label: "收藏" },
    { id: "recent" as const, label: "最近观看" },
    ...groups.map((group) => ({
      id: "group" as const,
      label: group.name,
      groupName: group.name,
      count: group.channelCount,
    })),
  ];
}
