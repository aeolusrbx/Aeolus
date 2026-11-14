import { describe, expect, it } from "vitest";
import { INDEX, search, settingsEntries } from "../src/search";

describe("search index", () => {
  it("reads sections, groups and fields from the Settings page", () => {
    const e = settingsEntries(`
      {section === "Launch" && (<Group title="Discord"><Field label="Rich Presence" description="Shows \\"Aeolus\\""></Field></Group>)}
      {section === "Advanced" && pathRow("Logs folder", x)}`);
    expect(e.map((x) => x.title)).toEqual(["Launch", "Rich Presence", "Advanced", "Logs folder"]);
    expect(e[1]).toMatchObject({ page: "settings", section: "Launch", setting: "Rich Presence", path: ["Settings", "Launch", "Discord"] });
  });

  it("indexes every real setting", () => {
    const titles = INDEX.map((e) => e.title);
    for (const t of ["FPS cap", "After a disconnect", "Theme", "Launch at Windows startup", "Debug logging", "Handle Roblox links"]) expect(titles).toContain(t);
  });

  it("ranks the best match first", () => {
    expect(search("fps")[0].title).toBe("FPS cap");
    expect(search("discord")[0].title).toMatch(/Discord|Rich Presence/);
    expect(search("mods")[0]).toMatchObject({ title: "Mods", kind: "page" });
    expect(search("rejoin").some((r) => r.title === "After a disconnect")).toBe(true);
    expect(search("")).toEqual([]);
  });
});
