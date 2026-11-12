import { Gamepad2, MapPin, Play, RotateCw } from "lucide-react";
import { useState } from "react";
import { api, errorMessage, isCancelled } from "../services/api";
import { useToast } from "../stores/toasts";
import { useApp } from "../stores/app";
import type { GameVisit } from "../types";
import { formatDuration, relativeTime } from "../utils/format";
import { Badge, Button, cx } from "./ui";

export function GameIcon({ game, size = 40 }: { game: GameVisit; size?: number }) {
  const [broken, setBroken] = useState(false);
  return game.iconUrl && !broken ? (
    <img src={game.iconUrl} alt="" width={size} height={size} onError={() => setBroken(true)} className="shrink-0 rounded-lg object-cover" style={{ width: size, height: size }} draggable={false} />
  ) : (
    <div className="flex shrink-0 items-center justify-center rounded-lg bg-surface-hover text-fg-3" style={{ width: size, height: size }}>
      <Gamepad2 size={size * 0.45} />
    </div>
  );
}

export function gameName(g: GameVisit) {
  return g.name ?? `Place ${g.placeId}`;
}

function duration(g: GameVisit) {
  if (!g.endedAt) return null;
  return formatDuration((new Date(g.endedAt).getTime() - new Date(g.startedAt).getTime()) / 1000);
}

export function GameRow({ game, live, compact }: { game: GameVisit; live?: boolean; compact?: boolean }) {
  const toast = useToast();
  const { progress } = useApp();
  const [busy, setBusy] = useState<"rejoin" | "play" | null>(null);
  const busyOp = !!progress && !progress.done;
  const go = async (sameServer: boolean) => {
    setBusy(sameServer ? "rejoin" : "play");
    try {
      await api.gameRejoin(game.id, sameServer);
      toast.success(sameServer ? `Rejoining ${gameName(game)}` : `Launching ${gameName(game)}`);
    } catch (e) {
      if (!isCancelled(e)) toast.error(sameServer ? "Couldn't rejoin" : "Couldn't launch", errorMessage(e));
    } finally {
      setBusy(null);
    }
  };
  const d = duration(game);
  return (
    <div className={cx("flex items-center gap-3", compact ? "px-4 py-2.5" : "px-4 py-3")}>
      <GameIcon game={game} size={compact ? 36 : 40} />
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <span className="truncate text-[13px] font-medium text-fg">{gameName(game)}</span>
          {live && <Badge tone="success">Playing now</Badge>}
          {game.serverKind !== "public" && <Badge>{game.serverKind === "private" ? "Private server" : "Reserved server"}</Badge>}
        </div>
        <div className="flex flex-wrap items-center gap-x-2 text-xs text-fg-3">
          {game.creator && <span className="truncate">by {game.creator}</span>}
          {game.location && (
            <span className="inline-flex items-center gap-1"><MapPin size={11} />{game.location}</span>
          )}
          <span>{live ? `since ${relativeTime(game.startedAt)}` : `${relativeTime(game.startedAt)}${d ? ` · ${d}` : ""}`}</span>
        </div>
      </div>
      {!live && (
        <div className="flex shrink-0 gap-1.5">
          {game.serverKind === "public" && (
            <Button size="sm" icon={<RotateCw size={12} />} loading={busy === "rejoin"} disabled={busyOp || !!busy} onClick={() => go(true)} title="Join this exact server again">
              Rejoin
            </Button>
          )}
          <Button size="sm" variant="ghost" icon={<Play size={12} />} loading={busy === "play"} disabled={busyOp || !!busy} onClick={() => go(false)} title="Join this game in any server">
            Play
          </Button>
        </div>
      )}
    </div>
  );
}
