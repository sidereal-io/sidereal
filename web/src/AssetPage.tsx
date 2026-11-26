import { useEffect, useState, type ReactNode } from "react";
import { InBar } from "./Bar.tsx";
import { imageUrlOf } from "./entry.ts";
import { Link } from "./navigation.tsx";
import "./AssetPage.css";

interface Fact {
  label: string;
  value: number;
  unit: string;
}

interface Asset {
  name: string;
  // The address the image shows from: its preview, or else its original.
  imageUrl: string;
  // The current version's media type, such as `image/png`.
  mediaType: string;
  facts: Fact[];
}

type Page = "loading" | "not-found" | "failed" | Asset;

// A request that the server refused because the session has ended.
const sessionEnded = Symbol("session ended");

// Grouped with commas in every locale, as DESIGN.md shows the values.
const numbers = new Intl.NumberFormat("en-US");

function isObject(body: unknown): body is Record<string, unknown> {
  return typeof body === "object" && body !== null;
}

// The common names of the media types this page knows. Any other media type
// shows as the server gives it.
const formats: Record<string, string> = {
  "image/png": "PNG",
  "image/jpeg": "JPEG",
  "image/tiff": "TIFF",
};

function formatOf(mediaType: string): string {
  return Object.hasOwn(formats, mediaType) ? formats[mediaType] : mediaType;
}

// The facts of a facet list, or null when any item is not a fact. Only the
// label, value and unit are kept: the page shows no provenance.
function facts(list: unknown): Fact[] | null {
  if (!Array.isArray(list)) return null;
  const found: Fact[] = [];
  for (const item of list as unknown[]) {
    if (
      !isObject(item) ||
      typeof item.name !== "string" ||
      typeof item.label !== "string" ||
      typeof item.unit !== "string" ||
      typeof item.value !== "number" ||
      !Number.isSafeInteger(item.value) ||
      item.value < 0
    ) {
      return null;
    }
    found.push({ label: item.label, value: item.value, unit: item.unit });
  }
  return found;
}

// The Asset in a body, or null when the body is not this page's Asset with
// its media type and facets. The image is fetched only from this Asset's own
// preview route, or else its original route, and the name, media type and
// labels are only ever shown as text.
function assetOf(body: unknown, id: string): Asset | null {
  if (!isObject(body) || !isObject(body.asset)) return null;
  const { asset } = body;
  const original = `/api/assets/${id}/original`;
  if (
    asset.id !== id ||
    typeof asset.name !== "string" ||
    asset.original_url !== original ||
    typeof asset.media_type !== "string" ||
    asset.media_type === ""
  ) {
    return null;
  }
  const list = facts(asset.facets);
  if (!list) return null;
  const imageUrl = imageUrlOf(id, asset.preview_url);
  if (imageUrl === null) return null;
  return {
    name: asset.name,
    imageUrl,
    mediaType: asset.media_type,
    facts: list,
  };
}

async function fetchAsset(
  id: string,
  signal: AbortSignal,
): Promise<Page | typeof sessionEnded> {
  try {
    const response = await fetch(`/api/assets/${id}`, { signal });
    if (response.status === 401) return sessionEnded;
    if (response.status === 404) return "not-found";
    if (response.status !== 200) return "failed";
    return assetOf(await response.json(), id) ?? "failed";
  } catch {
    return "failed";
  }
}

function LibraryLink({ className }: { className?: string }) {
  return (
    <Link className={className} to="/">
      Library
    </Link>
  );
}

// The image, whole and uncropped. When the browser cannot show it, a 3:2
// frame says so, and the name and facts stay beside it.
function Picture({ asset }: { asset: Asset }) {
  const [broken, setBroken] = useState(false);
  return (
    <figure className="asset-frame">
      {broken ? (
        <p className="asset-broken">This image could not be displayed.</p>
      ) : (
        <img
          className="asset-image"
          src={asset.imageUrl}
          alt={asset.name}
          onError={() => {
            setBroken(true);
          }}
        />
      )}
    </figure>
  );
}

// The facts: the Format line first, from the media type, then one line per
// facet in the server's order.
function Details({ asset }: { asset: Asset }) {
  return (
    <section className="asset-details" aria-label="Details">
      <h2 className="asset-name">{asset.name}</h2>
      <dl className="asset-facts">
        <div className="asset-fact">
          <dt>Format</dt>
          <dd>{formatOf(asset.mediaType)}</dd>
        </div>
        {asset.facts.map((fact, index) => (
          // Two values of one facet may share a label, so the list's own
          // order keys them. The list never reorders while it shows.
          // eslint-disable-next-line @eslint-react/no-array-index-key
          <div className="asset-fact" key={index}>
            <dt>{fact.label}</dt>
            <dd>
              {numbers.format(fact.value)}
              {fact.unit !== "" && ` ${fact.unit}`}
            </dd>
          </div>
        ))}
      </dl>
    </section>
  );
}

// One Asset at its own address, with the facts Sidereal has recorded about
// it, in the Side column layout that DESIGN.md describes. Render it with
// `key={id}`, so another Asset starts from `Loading the image…`.
function AssetPage({
  id,
  onSessionLost,
  notice,
  children,
}: {
  // The Asset's id, from the page's address.
  id: string;
  // Called when the server says the session has ended.
  onSessionLost: () => void;
  // A fixed message from the account, such as a failed sign-out.
  notice?: string | null;
  // The account's items, shown at the end of the top bar.
  children?: ReactNode;
}) {
  const [page, setPage] = useState<Page>("loading");
  // Each change asks the server again.
  const [generation, setGeneration] = useState(0);

  useEffect(() => {
    // Aborted when the page goes away or asks again, so a late answer never
    // shows data the user can no longer see.
    const controller = new AbortController();
    void fetchAsset(id, controller.signal).then((result) => {
      if (controller.signal.aborted) return;
      if (result === sessionEnded) onSessionLost();
      else setPage(result);
    });
    return () => {
      controller.abort();
    };
  }, [id, generation, onSessionLost]);

  function retry() {
    setPage("loading");
    setGeneration((value) => value + 1);
  }

  return (
    <article className="asset-page" aria-label="Asset">
      <InBar at="start">
        <LibraryLink className="asset-crumb" />
      </InBar>
      <InBar at="end">{children}</InBar>
      {notice && <p className="asset-status">{notice}</p>}
      {page === "loading" && <p className="asset-status">Loading the image…</p>}
      {page === "not-found" && (
        <p className="asset-status">
          This image is not in the library. <LibraryLink />
        </p>
      )}
      {page === "failed" && (
        <p className="asset-status">
          The image could not be loaded.{" "}
          <button className="button-quiet" type="button" onClick={retry}>
            Retry
          </button>
        </p>
      )}
      {typeof page === "object" && (
        <div className="asset">
          <Picture asset={page} />
          <Details asset={page} />
        </div>
      )}
    </article>
  );
}

export default AssetPage;
