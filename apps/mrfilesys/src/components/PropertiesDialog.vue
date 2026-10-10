<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import { Check, ClipboardCopy, Copy, Files, Fingerprint, LoaderCircle, SquareArrowOutUpRight, X } from "@lucide/vue";
import EntryIcon from "./EntryIcon.vue";
import Dialog from "@mrtools/ui/components/Dialog";
import Tip from "@mrtools/ui/components/Tip";
import { toast, toastError } from "@mrtools/ui/lib/toast";
import { api } from "@/lib/api";
import { copyPaths, openWith, showWindowsProperties } from "@/lib/actions";
import { previewOf, typeLabel } from "@/lib/fileTypes";
import { formatBytes, formatCount } from "@/lib/format";
import { parentPath } from "@/lib/paths";
import { navigate, refresh, state } from "@/lib/store";
import type { Entry, Hashes, ItemInfo, Measure } from "@/lib/types";

const open = computed({
  get: () => state.properties.length > 0,
  set: (value) => !value && (state.properties = []),
});

const entries = ref<Entry[]>([]);
const info = ref<ItemInfo | null>(null);
const measure = ref<Measure | null>(null);
const measuring = ref(false);
const hashes = ref<Hashes | null>(null);
const hashing = ref(false);
const expected = ref("");
const name = ref("");
const dimensions = ref("");

const single = computed(() => (entries.value.length === 1 ? entries.value[0] : undefined));
const parents = computed(() => [...new Set(entries.value.map((e) => parentPath(e.path)))]);
const folderCount = computed(() => entries.value.filter((e) => e.isDir).length);
/** Only files: their size is known without walking anything. */
const onlyFiles = computed(() => entries.value.every((e) => !e.isDir));

/** Loads everything for the paths in `state.properties`; `token` drops answers for an older opening. */
let token = 0;
async function load(paths: string[]) {
  const mine = ++token;
  info.value = measure.value = hashes.value = null;
  expected.value = dimensions.value = "";
  try {
    const loaded = await Promise.all(paths.map((p) => api.stat(p)));
    if (mine !== token) return;
    entries.value = loaded;
  } catch (e) {
    toastError("Eigenschaften konnten nicht gelesen werden", e);
    state.properties = [];
    return;
  }
  name.value = single.value?.name ?? "";
  if (single.value) {
    api.itemInfo(single.value.path).then((i) => mine === token && (info.value = i)).catch(() => {});
    if (previewOf(single.value) === "image") readDimensions(single.value.path, mine);
  }
  if (!onlyFiles.value || entries.value.length > 1) {
    measuring.value = true;
    try {
      const result = await api.measure(paths);
      if (mine === token) measure.value = result;
    } catch (e) {
      toastError("Größe konnte nicht berechnet werden", e);
    } finally {
      if (mine === token) measuring.value = false;
    }
  }
}

watch(
  () => state.properties,
  (paths) => paths.length && load(paths),
);

function readDimensions(path: string, mine: number) {
  const img = new Image();
  img.onload = () => mine === token && (dimensions.value = `${img.naturalWidth} × ${img.naturalHeight} Pixel`);
  img.src = convertFileSrc(path);
}

function fullDate(ms: number) {
  if (!ms) return "–";
  return new Date(ms).toLocaleString("de-DE", { dateStyle: "full", timeStyle: "medium" });
}

// ---------------------------------------------------------------------------
// Changes

async function rename() {
  const entry = single.value;
  if (!entry || !name.value.trim() || name.value === entry.name) return;
  try {
    const renamed = await api.rename(entry.path, name.value);
    state.properties = [renamed];
    await refresh({ quiet: true, select: [renamed] });
  } catch (e) {
    name.value = entry.name;
    toastError("Umbenennen fehlgeschlagen", e);
  }
}

/** true: all set, false: none, null: mixed. */
function attribute(key: "readonly" | "hidden"): boolean | null {
  const set = entries.value.filter((e) => e[key]).length;
  return set === 0 ? false : set === entries.value.length ? true : null;
}

async function setAttribute(key: "readonly" | "hidden", value: boolean) {
  const paths = entries.value.map((e) => e.path);
  try {
    await api.setAttributes(paths, { [key]: value });
  } catch (e) {
    toastError("Attribut konnte nicht geändert werden", e);
  }
  entries.value = await Promise.all(paths.map((p) => api.stat(p))).catch(() => entries.value);
  refresh({ quiet: true });
}

function go(path: string, select?: string) {
  state.properties = [];
  navigate(path, { select });
}

// ---------------------------------------------------------------------------
// Checksums

async function computeHashes() {
  const entry = single.value;
  if (!entry) return;
  const mine = token;
  hashing.value = true;
  try {
    const result = await api.hashes(entry.path);
    if (mine === token) hashes.value = result;
  } catch (e) {
    toastError("Prüfsumme konnte nicht berechnet werden", e);
  } finally {
    hashing.value = false;
  }
}

const hashRows = computed(() =>
  hashes.value
    ? [
        { label: "SHA-256", value: hashes.value.sha256 },
        { label: "SHA-1", value: hashes.value.sha1 },
        { label: "MD5", value: hashes.value.md5 },
      ]
    : [],
);
const normalizedExpected = computed(() => expected.value.trim().toLowerCase().replace(/\s/g, ""));
const matchingHash = computed(() => hashRows.value.find((h) => h.value === normalizedExpected.value)?.label);

async function copy(text: string, what: string) {
  try {
    await navigator.clipboard.writeText(text);
    toast("success", `${what} kopiert`);
  } catch (e) {
    toastError("Kopieren fehlgeschlagen", e);
  }
}

const title = computed(() =>
  single.value ? single.value.name || single.value.path : `${formatCount(entries.value.length)} Elemente`,
);
const description = computed(() => {
  if (single.value) return typeLabel(single.value);
  const files = entries.value.length - folderCount.value;
  return `${formatCount(folderCount.value)} Ordner, ${formatCount(files)} Dateien`;
});
const iconSrc = computed(() =>
  single.value && previewOf(single.value) === "image" ? convertFileSrc(single.value.path) : "",
);
</script>

<template>
  <Dialog v-model:open="open" :title="title" :description="description" size="md">
    <template #icon>
      <div class="grid size-10 shrink-0 place-items-center overflow-hidden rounded-lg bg-accent/10">
        <img v-if="iconSrc" :src="iconSrc" alt="" class="size-full object-cover" />
        <EntryIcon v-else-if="single" :entry="single" class="size-6" />
        <Files v-else class="size-6 text-accent-text" />
      </div>
    </template>

    <div v-if="entries.length" class="flex flex-col gap-4 text-[13px]">
      <!-- General -->
      <dl class="grid grid-cols-[8.5rem_1fr] items-baseline gap-x-3 gap-y-2 [&>dd]:min-w-0 [&>dd]:break-words [&>dt]:text-muted-foreground">
        <template v-if="single">
          <dt>Name</dt>
          <dd>
            <input
              v-model="name"
              class="input h-7"
              spellcheck="false"
              aria-label="Name"
              @keydown.enter="($event.target as HTMLInputElement).blur()"
              @keydown.esc.stop="name = single.name"
              @blur="rename"
            />
          </dd>
        </template>

        <dt>Ort</dt>
        <dd>
          <button
            v-if="parents.length === 1 && parents[0]"
            class="text-left font-mono text-xs text-accent-text hover:underline"
            @click="go(parents[0], single?.path)"
          >
            {{ parents[0] }}
          </button>
          <span v-else-if="parents.length > 1" class="text-muted-foreground">verschiedene Ordner</span>
          <span v-else class="text-muted-foreground">Laufwerk</span>
        </dd>

        <template v-if="single && !single.isDir">
          <dt>Öffnen mit</dt>
          <dd class="flex items-center gap-2">
            <span class="truncate">{{ info?.openWith ?? "–" }}</span>
            <button class="btn btn-outline h-6 px-2 text-xs" @click="openWith(single.path)">Ändern …</button>
          </dd>
        </template>

        <template v-if="info?.linkTarget">
          <dt>Verknüpft mit</dt>
          <dd>
            <button
              class="text-left font-mono text-xs text-accent-text hover:underline"
              @click="single?.isDir ? go(info.linkTarget) : go(parentPath(info.linkTarget), info.linkTarget)"
            >
              {{ info.linkTarget }}
            </button>
          </dd>
        </template>

        <template v-if="dimensions">
          <dt>Abmessungen</dt>
          <dd class="tabular-nums">{{ dimensions }}</dd>
        </template>

        <dt>Größe</dt>
        <dd class="tabular-nums">
          <template v-if="measure">
            {{ formatBytes(measure.size) }} <span class="text-muted-foreground">({{ formatCount(measure.size) }} Bytes)</span>
          </template>
          <template v-else-if="onlyFiles && single">
            {{ formatBytes(single.size) }} <span class="text-muted-foreground">({{ formatCount(single.size) }} Bytes)</span>
          </template>
          <span v-else-if="measuring" class="inline-flex items-center gap-1.5 text-muted-foreground">
            <LoaderCircle class="size-3.5 animate-spin" />wird berechnet …
          </span>
        </dd>

        <dt>Auf dem Datenträger</dt>
        <dd class="tabular-nums">
          <template v-if="measure">{{ formatBytes(measure.onDisk) }}</template>
          <template v-else-if="info?.onDisk != null">{{ formatBytes(info.onDisk) }}</template>
          <span v-else class="text-muted-foreground">–</span>
        </dd>

        <template v-if="measure && (measure.dirs || folderCount || entries.length > 1)">
          <dt>Inhalt</dt>
          <dd class="tabular-nums">
            {{ formatCount(measure.files) }} Dateien, {{ formatCount(measure.dirs) }} Ordner
            <span v-if="measure.errors" class="text-amber-600 dark:text-amber-400">
              · {{ formatCount(measure.errors) }} nicht lesbar
            </span>
          </dd>
        </template>

        <template v-if="single">
          <dt>Erstellt</dt>
          <dd class="tabular-nums">{{ fullDate(single.created) }}</dd>
          <dt>Geändert</dt>
          <dd class="tabular-nums">{{ fullDate(single.modified) }}</dd>
          <dt>Letzter Zugriff</dt>
          <dd class="tabular-nums">{{ fullDate(info?.accessed ?? 0) }}</dd>
        </template>

        <dt>Attribute</dt>
        <dd class="flex flex-wrap items-center gap-x-4 gap-y-1">
          <label v-for="key in ['readonly', 'hidden'] as const" :key="key" class="inline-flex cursor-pointer items-center gap-1.5">
            <input
              type="checkbox"
              class="size-3.5 accent-[var(--accent)]"
              :checked="attribute(key) === true"
              :indeterminate="attribute(key) === null"
              @change="setAttribute(key, ($event.target as HTMLInputElement).checked)"
            />
            {{ key === "readonly" ? "Schreibgeschützt" : "Versteckt" }}
          </label>
          <span v-if="entries.some((e) => e.system)" class="text-xs text-muted-foreground">Systemdatei</span>
        </dd>
      </dl>

      <!-- Checksums -->
      <section v-if="single && !single.isDir" class="rounded-lg border border-border p-3">
        <div class="flex items-center gap-2">
          <Fingerprint class="size-4 text-accent-text" />
          <h3 class="flex-1 font-medium">Prüfsummen</h3>
          <button v-if="!hashes" class="btn btn-outline h-7 text-xs" :disabled="hashing" @click="computeHashes">
            <LoaderCircle v-if="hashing" class="animate-spin" />
            {{ hashing ? "Wird berechnet …" : "Berechnen" }}
          </button>
        </div>
        <template v-if="hashes">
          <dl class="mt-2 grid grid-cols-[4.5rem_1fr_auto] items-center gap-x-2 gap-y-1">
            <template v-for="row in hashRows" :key="row.label">
              <dt class="text-xs text-muted-foreground">{{ row.label }}</dt>
              <dd
                class="truncate font-mono text-[11px] select-text"
                :class="matchingHash === row.label && 'font-semibold text-emerald-600 dark:text-emerald-400'"
                :title="row.value"
              >
                {{ row.value }}
              </dd>
              <Tip :text="`${row.label} kopieren`">
                <button class="icon-btn size-6 [&_svg]:size-3.5" :aria-label="`${row.label} kopieren`" @click="copy(row.value, row.label)">
                  <Copy />
                </button>
              </Tip>
            </template>
          </dl>
          <div class="mt-2 flex items-center gap-2">
            <input
              v-model="expected"
              class="input h-7 font-mono text-[11px]"
              placeholder="Erwartete Prüfsumme zum Vergleichen einfügen"
              spellcheck="false"
            />
            <span
              v-if="normalizedExpected"
              class="inline-flex shrink-0 items-center gap-1 text-xs font-medium"
              :class="matchingHash ? 'text-emerald-600 dark:text-emerald-400' : 'text-red-500'"
            >
              <template v-if="matchingHash"><Check class="size-3.5" />{{ matchingHash }} stimmt</template>
              <template v-else><X class="size-3.5" />stimmt nicht</template>
            </span>
          </div>
        </template>
      </section>
    </div>
    <div v-else class="grid h-32 place-items-center"><LoaderCircle class="size-5 animate-spin text-muted-foreground" /></div>

    <template #footer>
      <button class="btn btn-ghost h-7 text-xs" @click="copyPaths(entries.map((e) => e.path))">
        <ClipboardCopy />{{ entries.length > 1 ? "Pfade kopieren" : "Pfad kopieren" }}
      </button>
      <button v-if="single" class="btn btn-ghost h-7 text-xs" @click="showWindowsProperties(single.path)">
        <SquareArrowOutUpRight />Windows-Eigenschaften …
      </button>
      <div class="flex-1" />
      <button class="btn btn-primary h-7" @click="state.properties = []">Schließen</button>
    </template>
  </Dialog>
</template>
