<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from "vue";
import { create, type TextEditor } from "@mrtools/textedit";
import { theme } from "@mrtools/ui/lib/theme";

/** Formatierter Text (Vor-/Nachtext, Briefe) mit dem gemeinsamen Editor; liefert einfaches HTML. */
const model = defineModel<string>({ required: true });
const props = defineProps<{ placeholder?: string; readOnly?: boolean; hoch?: boolean }>();

const host = ref<HTMLElement>();
let editor: TextEditor | undefined;
let eigen = "";

onMounted(() => {
  editor = create(host.value!, {
    content: model.value,
    toolbar: ["bold", "italic", "underline", "|", "bulletList", "orderedList", "|", "undo", "redo"],
    placeholder: props.placeholder,
    readOnly: props.readOnly,
    theme: theme.value,
    spellcheck: true,
    onChange: () => {
      eigen = editor!.isEmpty ? "" : editor!.getHTML();
      model.value = eigen;
    },
  });
});

watch(model, (html) => {
  if (editor && html !== eigen) {
    eigen = html;
    editor.setHTML(html);
  }
});
watch(theme, (t) => editor?.setConfig({ theme: t }));
watch(
  () => props.readOnly,
  (r) => editor?.setReadOnly(!!r),
);

onBeforeUnmount(() => editor?.destroy());

/** Text an der Cursorposition einfügen (Textbausteine). */
function einfuegen(html: string) {
  editor?.insertHTML(html);
  editor?.focus();
}
defineExpose({ einfuegen });
</script>

<template>
  <div ref="host" class="richtext overflow-hidden rounded-md border border-border" :class="hoch ? 'richtext-hoch' : ''" />
</template>

<style>
.richtext .mrte-content,
.richtext .ProseMirror {
  min-height: 4.5rem;
  max-height: 16rem;
  overflow-y: auto;
}
.richtext-hoch .mrte-content,
.richtext-hoch .ProseMirror {
  min-height: 22rem;
  max-height: none;
}
</style>
