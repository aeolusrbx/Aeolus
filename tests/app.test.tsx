import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor, waitForElementToBeRemoved, within } from "@testing-library/react";
import App from "../src/App";
import { createState, installMock, type MockState } from "./mockBackend";

let state: MockState;

beforeEach(() => {
  state = installMock(createState());
  document.documentElement.removeAttribute("data-theme");
});
afterEach(() => {
  cleanup();
});

const nav = () => screen.getByRole("navigation", { name: "Main" });
const go = (label: string) => fireEvent.click(within(nav()).getByRole("button", { name: label }));

describe("Aeolus UI", () => {
  it("loads the dashboard with real backend data and reports ready", async () => {
    render(<App />);
    expect(await screen.findByText("Welcome back")).toBeInTheDocument();
    expect(await screen.findByText("Launched Roblox")).toBeInTheDocument();
    expect(screen.getAllByText("0.701.0.7010921").length).toBeGreaterThan(0);
    expect(screen.getByText("v0.1.0")).toBeInTheDocument();
    await waitFor(() => expect(state.calls.some((c) => c.cmd === "app_ready")).toBe(true));
  });

  it("navigates between every page from the sidebar", async () => {
    render(<App />);
    await screen.findByText("Welcome back");
    const pages: [string, string][] = [
      ["Installations", "Aeolus installation"],
      ["Launch", "Launching with"],
      ["Fast Flags", "Profiles"],
      ["Customization", "Accent color"],
      ["Plugins", "Graphics Presets"],
      ["Diagnostics", "Export report"],
      ["Settings", "Launch at Windows startup"],
      ["Home", "Welcome back"],
    ];
    for (const [label, marker] of pages) {
      go(label);
      expect(await screen.findByText(marker, {}, { timeout: 3000 })).toBeInTheDocument();
      expect(within(nav()).getByRole("button", { name: label })).toHaveAttribute("aria-current", "page");
    }
  });

  it("launches Roblox from the dashboard and records the launch", async () => {
    render(<App />);
    fireEvent.click(await screen.findByRole("button", { name: /Update & Launch/ }));
    expect(await screen.findByText("Roblox launched")).toBeInTheDocument();
    expect(state.calls.filter((c) => c.cmd === "launch_roblox")).toHaveLength(1);
    expect(state.launches).toHaveLength(1);
  });

  it("asks for confirmation before clearing activity", async () => {
    render(<App />);
    await screen.findByText("Launched Roblox");
    fireEvent.click(screen.getByRole("button", { name: "Clear" }));
    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText("Clear recent activity?")).toBeInTheDocument();
    fireEvent.click(within(dialog).getByRole("button", { name: "Cancel" }));
    await waitForElementToBeRemoved(() => screen.queryByRole("dialog"));
    expect(state.calls.some((c) => c.cmd === "activity_clear")).toBe(false);
    fireEvent.click(screen.getByRole("button", { name: "Clear" }));
    fireEvent.click(within(await screen.findByRole("dialog")).getByRole("button", { name: "Clear" }));
    await waitFor(() => expect(state.calls.some((c) => c.cmd === "activity_clear")).toBe(true));
    expect(await screen.findByText("No activity yet")).toBeInTheDocument();
  });

  it("persists a toggled setting through the backend", async () => {
    render(<App />);
    await screen.findByText("Welcome back");
    go("Settings");
    await screen.findByText("Start minimized");
    const row = screen.getByText("Start minimized").closest("div")!.parentElement!.parentElement!;
    fireEvent.click(within(row).getByRole("switch"));
    await waitFor(() => expect(state.settings.general.startMinimized).toBe(true));
  });

  it("switches theme and accent color", async () => {
    render(<App />);
    await screen.findByText("Welcome back");
    go("Customization");
    fireEvent.click(await screen.findByRole("button", { name: /Light/ }));
    await waitFor(() => expect(document.documentElement.dataset.theme).toBe("light"));
    expect(state.settings.general.theme).toBe("light");
    fireEvent.click(screen.getByRole("button", { name: "Accent #3E8EF7" }));
    await waitFor(() => expect(state.settings.general.accent).toBe("#3E8EF7"));
    expect(document.documentElement.style.getPropertyValue("--accent")).toBe("#3E8EF7");
    // invalid hex typed by hand is not saved
    const hex = screen.getByLabelText("Accent hex value");
    fireEvent.change(hex, { target: { value: "orange" } });
    fireEvent.blur(hex);
    expect(state.settings.general.accent).toBe("#3E8EF7");
  });

  it("filters and searches Fast Flags and shows allowlist status", async () => {
    render(<App />);
    await screen.findByText("Welcome back");
    go("Fast Flags");
    fireEvent.click(await screen.findByRole("button", { name: /^Performance/ }));
    expect(await screen.findByText("DFIntTaskSchedulerTargetFps")).toBeInTheDocument();
    expect(screen.getByText("Not allowlisted")).toBeInTheDocument();
    expect(screen.getByText("Invalid")).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText("Search flags"), { target: { value: "texture" } });
    await waitFor(() => expect(screen.queryByText("DFIntTaskSchedulerTargetFps")).not.toBeInTheDocument());
    expect(screen.getByText("DFIntTextureQualityOverride")).toBeInTheDocument();
    fireEvent.change(screen.getByLabelText("Search flags"), { target: { value: "" } });
    fireEvent.change(screen.getByLabelText("Status filter"), { target: { value: "invalid" } });
    await waitFor(() => expect(screen.queryByText("DFIntTextureQualityOverride")).not.toBeInTheDocument());
    expect(screen.getByText("FFlagBadValue")).toBeInTheDocument();
  });

  it("adds a Fast Flag through the editor dialog", async () => {
    render(<App />);
    await screen.findByText("Welcome back");
    go("Fast Flags");
    fireEvent.click(await within(screen.getByRole("main")).findByRole("button", { name: /^Custom\s*0$/ }));
    fireEvent.click((await screen.findAllByRole("button", { name: "Add flag" }))[0]);
    const dialog = await screen.findByRole("dialog");
    fireEvent.change(within(dialog).getByPlaceholderText(/DFIntTextureQualityOverride/), { target: { value: "FIntDebugForceMSAASamples" } });
    fireEvent.change(within(dialog).getByRole("textbox"), { target: { value: "abc" } });
    expect(within(dialog).getByText("Enter a whole number")).toBeInTheDocument();
    fireEvent.change(within(dialog).getByRole("textbox"), { target: { value: "8" } });
    fireEvent.click(within(dialog).getByRole("button", { name: "Save" }));
    await waitFor(() => expect(state.flags.find((f) => f.id === "f4")!.flags.FIntDebugForceMSAASamples).toBe(8));
  });

  it("switches the active launch profile", async () => {
    render(<App />);
    await screen.findByText("Welcome back");
    go("Launch");
    fireEvent.click(await screen.findByRole("button", { name: "Make active" }));
    await waitFor(() => expect(state.profiles.activeId).toBe("p2"));
  });

  it("requires approval before enabling a plugin", async () => {
    render(<App />);
    await screen.findByText("Welcome back");
    go("Plugins");
    fireEvent.click(await screen.findByRole("button", { name: /Review & enable/ }));
    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText(/Run its hooks before and after Roblox launches/)).toBeInTheDocument();
    fireEvent.click(within(dialog).getByRole("button", { name: "Cancel" }));
    expect(state.calls.some((c) => c.cmd === "plugin_enable")).toBe(false);
  });

  it("filters logs by severity", async () => {
    render(<App />);
    await screen.findByText("Welcome back");
    go("Diagnostics");
    expect(await screen.findByText(/Updating Roblox failed/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "ERROR" }));
    await waitFor(() => expect(screen.queryByText(/Updating Roblox failed/)).not.toBeInTheDocument());
    expect(screen.getByText(/Roblox started/)).toBeInTheDocument();
  });
});
