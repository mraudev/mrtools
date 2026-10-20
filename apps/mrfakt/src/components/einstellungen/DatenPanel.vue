<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open, save } from "@tauri-apps/plugin-dialog";
import { ArchiveRestore, DatabaseBackup, FolderOpen, HardDriveDownload, LoaderCircle } from "@lucide/vue";
import { api } from "@/lib/api";
import { betrag, datum } from "@/lib/format";
import { frage } from "@/lib/nav";
import type { DbInfo, ImportBericht } from "@/lib/types";
import { toast, toastError } from "@mrtools/ui/lib/toast";

const sicherungen = defineModel<number>("sicherungen", { required: true });
const emit = defineEmits<{ importiert: [] }>();

const info = ref<DbInfo>();
const ordner = ref("");
const laeuft = ref(false);
const fortschritt = ref({ text: "", anteil: 0 });
const bericht = ref<ImportBericht | null>(null);
let abmelden: UnlistenFn | undefined;

async function laden() {
  info.value = await api.dbInfo().catch(() => undefined);
}

onMounted(async () => {
  laden();
  ordner.value = (await api.altdatenVorschlag().catch(() => null)) ?? "";
  abmelden = await listen<{ text: string; anteil: number }>("import-fortschritt", (e) => (fortschritt.value = e.payload));
});
onUnmounted(() => abmelden?.());

async function sichernJetzt() {
  try {
    const pfad = await api.sichern();
    toast("success", "Sicherung erstellt", pfad);
  } catch (e) {
    toastError("Sicherung fehlgeschlagen", e);
  }
}

async function wiederherstellen() {
  const quelle = await open({ defaultPath: info.value?.sicherungen, filters: [{ name: "mrfakt-Datenbank", extensions: ["db"] }] });
  if (typeof quelle !== "string") return;
  if (!(await frage("Sicherung zurückspielen", `Die aktuellen Daten werden durch\n${quelle}\nersetzt. Vorher wird automatisch eine Sicherung angelegt.`, "Zurückspielen"))) return;
  try {
    await api.wiederherstellen(quelle);
    toast("success", "Sicherung zurückgespielt");
    emit("importiert");
    laden();
  } catch (e) {
    toastError("Zurückspielen fehlgeschlagen", e);
  }
}

async function wechseln(neu: boolean) {
  const pfad = neu
    ? await save({ defaultPath: "mrfakt.db", filters: [{ name: "mrfakt-Datenbank", extensions: ["db"] }] })
    : await open({ filters: [{ name: "mrfakt-Datenbank", extensions: ["db"] }] });
  if (typeof pfad !== "string") return;
  try {
    await api.dbWechseln(pfad);
    toast("success", "Datenbank gewechselt", pfad);
    emit("importiert");
    laden();
  } catch (e) {
    toastError("Datenbank konnte nicht geöffnet werden", e);
  }
}

async function ordnerWaehlen() {
  const p = await open({ directory: true, defaultPath: ordner.value || undefined });
  if (typeof p === "string") ordner.value = p;
}

async function importieren() {
  if (!ordner.value) return;
  const vorhanden = (info.value?.kunden ?? 0) > 0;
  const text = vorhanden
    ? "Alle Daten in mrfakt werden durch die Daten aus GHFakt ersetzt (vorher wird automatisch gesichert). Die GHFakt-Dateien selbst werden nur gelesen."
    : "Kunden, Artikel, Projekte, alle Belege mit Positionen und Aufmaß, Regieberichte, Zahlungen und Einstellungen werden übernommen. Die GHFakt-Dateien werden nur gelesen.";
  if (!(await frage("Daten aus GHFakt übernehmen", text, "Übernehmen", vorhanden))) return;
  laeuft.value = true;
  bericht.value = null;
  fortschritt.value = { text: "Start …", anteil: 0 };
  try {
    bericht.value = await api.altdatenImportieren(ordner.value);
    toast("success", "Übernahme abgeschlossen");
    emit("importiert");
    laden();
  } catch (e) {
    toastError("Übernahme fehlgeschlagen", e);
  } finally {
    laeuft.value = false;
  }
}

const mb = (b: number) => (b / 1024 / 1024).toLocaleString("de-DE", { maximumFractionDigits: 1 }) + " MB";
</script>

<template>
  <div class="flex flex-col gap-4">
    <div class="rounded-xl border border-border bg-card p-4">
      <div class="label">Datenbank</div>
      <p v-if="info" class="text-[13px]">
        <span class="font-mono text-xs break-all">{{ info.pfad }}</span><br />
        <span class="text-muted-foreground">{{ mb(info.groesse) }} · {{ info.kunden.toLocaleString("de-DE") }} Kunden · {{ info.belege.toLocaleString("de-DE") }} Belege</span>
      </p>
      <div class="mt-3 flex flex-wrap gap-2">
        <button class="btn btn-outline h-8" @click="sichernJetzt"><DatabaseBackup />Jetzt sichern</button>
        <button class="btn btn-outline h-8" @click="wiederherstellen"><ArchiveRestore />Sicherung zurückspielen …</button>
        <button class="btn btn-outline h-8" @click="info && api.ordnerZeigen(info.sicherungen)"><FolderOpen />Sicherungsordner</button>
        <button class="btn btn-ghost h-8" @click="wechseln(false)">Andere Datenbank öffnen …</button>
        <button class="btn btn-ghost h-8" @click="wechseln(true)">Neue Datenbank anlegen …</button>
      </div>
      <label class="mt-3 flex items-center gap-2 text-[13px]">
        Beim Beenden automatisch sichern, die letzten
        <input v-model.number="sicherungen" type="number" min="1" class="input h-7 w-20" />
        Sicherungen behalten
      </label>
    </div>

    <div class="rounded-xl border border-border bg-card p-4">
      <div class="label">Daten aus GHFakt übernehmen</div>
      <p class="mb-3 text-xs text-muted-foreground">
        Liest Kunden, Artikel, PLZ, Rechnungsbuch (Access) sowie alle Projekte, Belege (Vorspann, Positionen, Nachspann, Abschläge), Aufmaße, Regieberichte, Briefe, Textbausteine,
        Formeln, Nummernkreise, Briefkopf und Mahntexte. Braucht den 32-bit-Jet-Treiber von Windows (ist auf jedem Windows vorhanden).
      </p>
      <div class="flex gap-2">
        <input v-model="ordner" class="input" placeholder="GHFakt-Ordner, z. B. C:\Entwicklung\GHFakt" />
        <button class="btn btn-outline h-8" @click="ordnerWaehlen"><FolderOpen />Wählen</button>
        <button class="btn btn-primary h-8" :disabled="!ordner || laeuft" @click="importieren"><HardDriveDownload />Übernehmen</button>
      </div>
      <div v-if="laeuft" class="mt-3">
        <div class="flex items-center gap-2 text-xs"><LoaderCircle class="size-3.5 animate-spin" />{{ fortschritt.text }}</div>
        <div class="mt-1 h-1.5 rounded bg-foreground/8"><div class="h-1.5 rounded bg-accent transition-all" :style="{ width: fortschritt.anteil * 100 + '%' }" /></div>
      </div>
    </div>

    <div v-if="bericht" class="rounded-xl border border-border bg-card p-4">
      <div class="label">Ergebnis ({{ bericht.dauer_s.toFixed(0) }} s)</div>
      <div class="grid grid-cols-2 gap-x-6 gap-y-0.5 text-[13px]">
        <div v-for="(n, k) in bericht.anzahl" :key="k" class="flex justify-between border-b border-border/40 py-0.5">
          <span class="text-muted-foreground">{{ k }}</span><span class="tabular-nums">{{ n.toLocaleString("de-DE") }}</span>
        </div>
      </div>
      <template v-if="bericht.abweichungen_gesamt">
        <div class="label mt-4">
          {{ bericht.abweichungen_gesamt }} Belege, deren neu berechneter Betrag vom GHFakt-Rechnungsbuch abweicht (im Rechnungsbuch gilt der Betrag aus GHFakt)
        </div>
        <div class="max-h-72 overflow-y-auto text-xs">
          <div v-for="a in bericht.abweichungen" :key="a.beleg" class="grid grid-cols-[5.5rem_1fr_10rem_6rem_6rem] gap-2 border-b border-border/40 py-0.5">
            <span class="tabular-nums">{{ datum(a.datum) }}</span>
            <span class="truncate">{{ a.beleg }}</span>
            <span class="truncate text-muted-foreground">{{ a.kunde }}</span>
            <span class="text-right tabular-nums">{{ betrag(a.berechnet) }}</span>
            <span class="text-right tabular-nums">{{ betrag(a.rechnungsbuch) }} {{ a.waehrung }}</span>
          </div>
        </div>
      </template>
      <template v-if="bericht.hinweise.length">
        <div class="label mt-4">Hinweise</div>
        <ul class="max-h-48 list-disc overflow-y-auto pl-5 text-xs text-muted-foreground">
          <li v-for="h in bericht.hinweise" :key="h">{{ h }}</li>
        </ul>
      </template>
    </div>
  </div>
</template>
