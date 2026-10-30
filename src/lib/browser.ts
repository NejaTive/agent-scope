import type {
  ArtifactKind,
  ConfigArtifact,
  DiscoveryStatus,
  Product,
  Scope,
  Timestamp,
} from "@/types/config";
import { resolveProduct } from "@/lib/labels";

/** Normalised row used by ArtifactBrowser / ArtifactDetail across all feature views. */
export interface BrowserItem {
  id: string;
  name: string;
  subtitle?: string | null;
  scope: Scope;
  status?: DiscoveryStatus;
  mtime?: Timestamp;
  path: string;
  projectPath?: string | null;
  /**
   * Filesystem path used to load content via `get_artifact_content`.
   * Falls back to `path` (never the artifact id).
   */
  artifactId?: string | null;
  /** Artifact kind (drives the optional kind filter + badge). */
  kind?: ArtifactKind | string;
  /** Claude Code vs Cursor; missing → treated as Claude Code. */
  product?: Product | null;
  /** Extra searchable text. */
  searchText?: string;
}

export function artifactToItem(a: ConfigArtifact): BrowserItem {
  return {
    id: a.id,
    name: a.name,
    subtitle: a.description ?? null,
    scope: a.scope,
    status: a.status,
    mtime: a.modifiedAt ?? null,
    path: a.path,
    projectPath: a.projectPath ?? null,
    artifactId: a.path,
    kind: a.kind,
    product: resolveProduct(a.product),
  };
}

/** Compare for stable list ordering: scope precedence then name. */
export function byName(a: BrowserItem, b: BrowserItem): number {
  return a.name.localeCompare(b.name, undefined, { sensitivity: "base" });
}
