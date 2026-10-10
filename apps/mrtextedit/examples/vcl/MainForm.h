// mrtextedit in einer VCL-Anwendung (C++Builder) über TEdgeBrowser.
// Neben die .exe gehören mrtextedit.html (aus packages/textedit/dist) und WebView2Loader.dll
// (aus dem Redist-Ordner von RAD Studio).
#ifndef MainFormH
#define MainFormH

#include <System.Classes.hpp>
#include <Vcl.Controls.hpp>
#include <Vcl.Edge.hpp>
#include <Vcl.ExtCtrls.hpp>
#include <Vcl.Forms.hpp>
#include <Vcl.StdCtrls.hpp>

class TFormMain : public TForm
{
private:
    TEdgeBrowser *FEditor;
    TMemo *FHtmlView;
    bool FReady;      // editor page loaded and ready for setHtml/setConfig
    String FHtml;     // current text as HTML, kept up to date by the "change"/"content" messages

    void __fastcall EditorWebMessageReceived(TCustomEdgeBrowser *Sender, TWebMessageReceivedEventArgs *Args);
    void __fastcall LoadClick(TObject *Sender);
    void __fastcall ShowClick(TObject *Sender);
    void SetEditorHtml(const String &html);
    void SetEditorConfig(const String &json);

public:
    __fastcall TFormMain(TComponent *Owner);
};

extern PACKAGE TFormMain *FormMain;

#endif
