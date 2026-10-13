#include <vcl.h>
#pragma hdrstop

#include <memory>
#include <System.IOUtils.hpp>
#include <System.JSON.hpp>
#include "MainForm.h"

#pragma package(smart_init)

TFormMain *FormMain;

// Toolbar and formats – the same JSON as in the web and in the mrtextedit app.
static const wchar_t *EDITOR_CONFIG = LR"({
  "toolbar": ["undo", "redo", "|", "paragraphFormat", "characterFormat", "|",
              "bold", "italic", "underline", "|", "alignLeft", "alignCenter", "alignRight", "|",
              "bulletList", "orderedList", "|", "link", "clearFormat"],
  "paragraphFormats": [
    { "label": "Standard" },
    { "label": "Überschrift", "tag": "h2" },
    { "label": "Zitat", "class": "zitat", "css": "font-style: italic; margin-left: 2em" }
  ],
  "characterFormats": [
    { "label": "Markiert", "class": "marker", "css": "background: #fde68a" }
  ],
  "theme": "light"
})";

// Form without .dfm: TForm(Owner, 0) creates it empty, all controls are made in code.
__fastcall TFormMain::TFormMain(TComponent *Owner)
    : TForm(Owner, 0), FReady(false)
{
    Caption = L"mrtextedit in VCL";
    Width = 1000;
    Height = 720;
    Position = poScreenCenter;

    TPanel *bar = new TPanel(this);
    bar->Parent = this;
    bar->Align = alTop;
    bar->Height = 40;
    bar->BevelOuter = bvNone;

    TButton *load = new TButton(this);
    load->Parent = bar;
    load->SetBounds(8, 8, 150, 25);
    load->Caption = L"Text hineingeben";
    load->OnClick = LoadClick;

    TButton *show = new TButton(this);
    show->Parent = bar;
    show->SetBounds(166, 8, 150, 25);
    show->Caption = L"HTML holen";
    show->OnClick = ShowClick;

    FHtmlView = new TMemo(this);
    FHtmlView->Parent = this;
    FHtmlView->Align = alBottom;
    FHtmlView->Height = 160;
    FHtmlView->ReadOnly = true;
    FHtmlView->ScrollBars = ssBoth;

    FEditor = new TEdgeBrowser(this);
    FEditor->Parent = this;
    FEditor->Align = alClient;
    // WebView2 needs a writable folder (not next to an .exe in "Program Files").
    FEditor->UserDataFolder = TPath::Combine(TPath::GetCachePath(), L"MeineApp\\WebView2");
    FEditor->OnWebMessageReceived = EditorWebMessageReceived;
    String page = ExtractFilePath(Application->ExeName) + L"mrtextedit.html";
    FEditor->Navigate(L"file:///" + StringReplace(page, L"\\", L"/", TReplaceFlags() << rfReplaceAll));
}

// Messages from the editor page: {"type":"ready"} and {"type":"change"|"content","html":"…","text":"…"}.
void __fastcall TFormMain::EditorWebMessageReceived(TCustomEdgeBrowser *Sender, TWebMessageReceivedEventArgs *Args)
{
    System::WideChar *raw = nullptr;
    if (FAILED(Args->ArgsInterface->TryGetWebMessageAsString(raw)) || raw == nullptr)
        return;
    String text = raw;
    CoTaskMemFree(raw);

    std::unique_ptr<TJSONValue> json(TJSONObject::ParseJSONValue(text));
    TJSONObject *message = dynamic_cast<TJSONObject *>(json.get());
    if (message == nullptr)
        return;
    TJSONValue *type = message->GetValue(L"type");
    if (type == nullptr)
        return;

    if (type->Value() == L"ready") {
        // Only now does the page accept commands – send configuration and initial text.
        FReady = true;
        SetEditorConfig(EDITOR_CONFIG);
        SetEditorHtml(L"<h2>Hallo aus VCL</h2><p>Dieser Text kommt aus <span class=\"marker\">C++</span>.</p>");
    } else if (type->Value() == L"change" || type->Value() == L"content") {
        TJSONValue *html = message->GetValue(L"html");
        FHtml = html ? html->Value() : String();
        if (type->Value() == L"change")
            FHtmlView->Text = FHtml;   // the user edited the text – e.g. set a "modified" flag here
    }
}

void TFormMain::SetEditorHtml(const String &html)
{
    FHtml = html;
    if (!FReady)
        return;   // sent on "ready"
    std::unique_ptr<TJSONString> literal(new TJSONString(html));
    FEditor->ExecuteScript(L"mrte.setHtml(" + literal->ToJSON() + L")");
}

void TFormMain::SetEditorConfig(const String &json)
{
    if (FReady)
        FEditor->ExecuteScript(L"mrte.setConfig(" + json + L")");
}

void __fastcall TFormMain::LoadClick(TObject *Sender)
{
    SetEditorHtml(L"<p class=\"zitat\">Ein neuer Text, gesetzt am " + Now().DateTimeString() + L".</p>");
}

void __fastcall TFormMain::ShowClick(TObject *Sender)
{
    FHtmlView->Text = FHtml;
}
