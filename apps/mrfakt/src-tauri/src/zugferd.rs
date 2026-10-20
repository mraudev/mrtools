//! E-Rechnung: Factur-X/ZUGFeRD 2 (Profil EN 16931, CII-Syntax) und die XMP-Angaben im PDF.

use crate::belege::BelegVoll;
use crate::einstellungen::Firma;
use crate::rechnen::{cent, gp};
use rust_decimal::Decimal;

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn b(d: Decimal) -> String {
    format!("{:.2}", cent(d))
}

fn datum(iso: &str) -> String {
    iso.replace('-', "")
}

/// Einheit nach UN/ECE Recommendation 20.
pub fn einheit(e: &str) -> &'static str {
    let e = e.trim().trim_end_matches('.').to_lowercase();
    match e.as_str() {
        "m²" | "m2" | "qm" => "MTK",
        "m" | "lfm" | "lfdm" | "stgm" => "MTR",
        "m³" | "m3" | "cbm" => "MTQ",
        "std" | "h" | "stunde" | "stunden" => "HUR",
        "st" | "stk" | "stück" | "stck" => "H87",
        "kg" => "KGM",
        "l" | "liter" | "ltr" => "LTR",
        "t" | "to" => "TNE",
        "pauschal" | "psch" | "pausch" | "pal" => "LS",
        "tag" | "tage" => "DAY",
        "woche" | "wochen" => "WEE",
        _ => "C62",
    }
}

/// Prüft, ob sich der Beleg als E-Rechnung abbilden lässt. Gibt den Grund zurück, wenn nicht.
pub fn moeglich(v: &BelegVoll, firma: &Firma) -> Result<(), String> {
    if !crate::belege::arten::art(&v.beleg.art).forderung {
        return Err("Nur Rechnungen und Gutschriften".into());
    }
    if v.beleg.waehrung != "EUR" {
        return Err("Nur Belege in Euro".into());
    }
    if !v.summen.zuschlaege_brutto.is_empty() {
        return Err("Zu-/Abschläge auf den Bruttobetrag sind in der E-Rechnung nicht vorgesehen".into());
    }
    if firma.name.trim().is_empty() || firma.ort.trim().is_empty() {
        return Err("Firmenname und Anschrift fehlen in den Einstellungen".into());
    }
    if firma.ustid.trim().is_empty() && firma.steuernummer.trim().is_empty() {
        return Err("Steuernummer oder USt-IdNr. fehlt in den Einstellungen".into());
    }
    Ok(())
}

pub fn xml(v: &BelegVoll, firma: &Firma) -> String {
    let beleg = &v.beleg;
    let kunde = v.kunde.clone().unwrap_or_default();
    let s = &v.summen;
    let gutschrift = beleg.art == "gutschrift";
    let typ = if gutschrift { "381" } else { "380" };
    let (kategorie, satz) = if beleg.netto_rechnung { ("AE", Decimal::ZERO) } else { ("S", beleg.mwst_satz) };
    let steuer = |inhalt: &str| format!("<ram:ApplicableTradeTax><ram:TypeCode>VAT</ram:TypeCode>{inhalt}<ram:CategoryCode>{kategorie}</ram:CategoryCode><ram:RateApplicablePercent>{}</ram:RateApplicablePercent></ram:ApplicableTradeTax>", b(satz));

    let mut x = String::new();
    x.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>
<rsm:CrossIndustryInvoice xmlns:rsm="urn:un:unece:uncefact:data:standard:CrossIndustryInvoice:100" xmlns:ram="urn:un:unece:uncefact:data:standard:ReusableAggregateBusinessInformationEntity:100" xmlns:qdt="urn:un:unece:uncefact:data:standard:QualifiedDataType:100" xmlns:udt="urn:un:unece:uncefact:data:standard:UnqualifiedDataType:100">
<rsm:ExchangedDocumentContext><ram:GuidelineSpecifiedDocumentContextParameter><ram:ID>urn:cen.eu:en16931:2017</ram:ID></ram:GuidelineSpecifiedDocumentContextParameter></rsm:ExchangedDocumentContext>
"#);
    x.push_str(&format!(
        "<rsm:ExchangedDocument><ram:ID>{}</ram:ID><ram:TypeCode>{typ}</ram:TypeCode><ram:IssueDateTime><udt:DateTimeString format=\"102\">{}</udt:DateTimeString></ram:IssueDateTime>",
        esc(&beleg.nummer),
        datum(&beleg.datum)
    ));
    if !beleg.titel.is_empty() {
        x.push_str(&format!("<ram:IncludedNote><ram:Content>{}</ram:Content></ram:IncludedNote>", esc(&beleg.titel)));
    }
    if beleg.netto_rechnung {
        x.push_str("<ram:IncludedNote><ram:Content>Steuerschuldnerschaft des Leistungsempfängers (§ 13b UStG)</ram:Content></ram:IncludedNote>");
    }
    x.push_str("</rsm:ExchangedDocument>\n<rsm:SupplyChainTradeTransaction>\n");

    // Positionen
    let mut nr = 0;
    for p in v.positionen.iter().filter(|p| p.art == "N") {
        nr += 1;
        let mut zeilen = p.text.lines();
        let kurz = zeilen.next().unwrap_or("").trim();
        let lang: Vec<&str> = zeilen.collect();
        let roh = cent(p.menge * p.ep);
        let summe = gp(p);
        x.push_str(&format!("<ram:IncludedSupplyChainTradeLineItem><ram:AssociatedDocumentLineDocument><ram:LineID>{nr}</ram:LineID></ram:AssociatedDocumentLineDocument>"));
        x.push_str(&format!(
            "<ram:SpecifiedTradeProduct>{}<ram:Name>{}</ram:Name>{}</ram:SpecifiedTradeProduct>",
            if p.posnr.is_empty() { String::new() } else { format!("<ram:SellerAssignedID>{}</ram:SellerAssignedID>", esc(&p.posnr)) },
            esc(if kurz.is_empty() { "Position" } else { kurz }),
            if lang.is_empty() { String::new() } else { format!("<ram:Description>{}</ram:Description>", esc(lang.join(" ").trim())) }
        ));
        x.push_str(&format!(
            "<ram:SpecifiedLineTradeAgreement><ram:NetPriceProductTradePrice><ram:ChargeAmount>{}</ram:ChargeAmount></ram:NetPriceProductTradePrice></ram:SpecifiedLineTradeAgreement>",
            p.ep.round_dp(4).normalize()
        ));
        x.push_str(&format!(
            "<ram:SpecifiedLineTradeDelivery><ram:BilledQuantity unitCode=\"{}\">{}</ram:BilledQuantity></ram:SpecifiedLineTradeDelivery>",
            einheit(&p.einheit),
            p.menge.round_dp(4).normalize()
        ));
        x.push_str("<ram:SpecifiedLineTradeSettlement>");
        x.push_str(&steuer(""));
        if roh != summe {
            x.push_str(&format!(
                "<ram:SpecifiedTradeAllowanceCharge><ram:ChargeIndicator><udt:Indicator>false</udt:Indicator></ram:ChargeIndicator><ram:ActualAmount>{}</ram:ActualAmount><ram:Reason>Rabatt {} %</ram:Reason></ram:SpecifiedTradeAllowanceCharge>",
                b(roh - summe),
                p.rabatt.normalize()
            ));
        }
        x.push_str(&format!("<ram:SpecifiedTradeSettlementLineMonetarySummation><ram:LineTotalAmount>{}</ram:LineTotalAmount></ram:SpecifiedTradeSettlementLineMonetarySummation></ram:SpecifiedLineTradeSettlement></ram:IncludedSupplyChainTradeLineItem>\n", b(summe)));
    }

    // Verkäufer / Käufer
    let adresse = |strasse: &str, plz: &str, ort: &str, land: &str| {
        format!(
            "<ram:PostalTradeAddress><ram:PostcodeCode>{}</ram:PostcodeCode><ram:LineOne>{}</ram:LineOne><ram:CityName>{}</ram:CityName><ram:CountryID>{}</ram:CountryID></ram:PostalTradeAddress>",
            esc(plz), esc(strasse), esc(ort), if land.trim().len() == 2 { land.trim().to_uppercase() } else { "DE".into() }
        )
    };
    x.push_str("<ram:ApplicableHeaderTradeAgreement>");
    if !kunde.leitweg_id.is_empty() {
        x.push_str(&format!("<ram:BuyerReference>{}</ram:BuyerReference>", esc(&kunde.leitweg_id)));
    }
    x.push_str(&format!("<ram:SellerTradeParty><ram:Name>{}</ram:Name>", esc(&firma.name)));
    if !firma.telefon.is_empty() || !firma.email.is_empty() {
        x.push_str(&format!("<ram:DefinedTradeContact><ram:PersonName>{}</ram:PersonName>", esc(if firma.inhaber.is_empty() { &firma.name } else { &firma.inhaber })));
        if !firma.telefon.is_empty() {
            x.push_str(&format!("<ram:TelephoneUniversalCommunication><ram:CompleteNumber>{}</ram:CompleteNumber></ram:TelephoneUniversalCommunication>", esc(&firma.telefon)));
        }
        if !firma.email.is_empty() {
            x.push_str(&format!("<ram:EmailURIUniversalCommunication><ram:URIID>{}</ram:URIID></ram:EmailURIUniversalCommunication>", esc(&firma.email)));
        }
        x.push_str("</ram:DefinedTradeContact>");
    }
    x.push_str(&adresse(&firma.strasse, &firma.plz, &firma.ort, &firma.land));
    if !firma.email.is_empty() {
        x.push_str(&format!("<ram:URIUniversalCommunication><ram:URIID schemeID=\"EM\">{}</ram:URIID></ram:URIUniversalCommunication>", esc(&firma.email)));
    }
    if !firma.steuernummer.is_empty() {
        x.push_str(&format!("<ram:SpecifiedTaxRegistration><ram:ID schemeID=\"FC\">{}</ram:ID></ram:SpecifiedTaxRegistration>", esc(&firma.steuernummer)));
    }
    if !firma.ustid.is_empty() {
        x.push_str(&format!("<ram:SpecifiedTaxRegistration><ram:ID schemeID=\"VA\">{}</ram:ID></ram:SpecifiedTaxRegistration>", esc(&firma.ustid)));
    }
    x.push_str("</ram:SellerTradeParty>");
    let kname = [kunde.name1.trim(), kunde.name2.trim()].iter().filter(|s| !s.is_empty()).cloned().collect::<Vec<_>>().join(" ");
    let kname = if kname.is_empty() { beleg.adresse.lines().nth(1).unwrap_or("Kunde").to_string() } else { kname };
    x.push_str(&format!("<ram:BuyerTradeParty>{}<ram:Name>{}</ram:Name>", if kunde.nummer.is_empty() { String::new() } else { format!("<ram:ID>{}</ram:ID>", esc(&kunde.nummer)) }, esc(&kname)));
    x.push_str(&adresse(&kunde.strasse, &kunde.plz, &kunde.ort, &kunde.land));
    if !kunde.email.is_empty() {
        x.push_str(&format!("<ram:URIUniversalCommunication><ram:URIID schemeID=\"EM\">{}</ram:URIID></ram:URIUniversalCommunication>", esc(&kunde.email)));
    }
    if !kunde.ustid.is_empty() {
        x.push_str(&format!("<ram:SpecifiedTaxRegistration><ram:ID schemeID=\"VA\">{}</ram:ID></ram:SpecifiedTaxRegistration>", esc(&kunde.ustid)));
    }
    x.push_str("</ram:BuyerTradeParty>");
    if let Some(p) = &v.projekt {
        if !p.nummer.is_empty() {
            x.push_str(&format!("<ram:SpecifiedProcuringProject><ram:ID>{}</ram:ID><ram:Name>{}</ram:Name></ram:SpecifiedProcuringProject>", esc(&p.nummer), esc(if p.bezeichnung.is_empty() { "Projekt" } else { &p.bezeichnung })));
        }
    }
    x.push_str("</ram:ApplicableHeaderTradeAgreement>\n");
    x.push_str(&format!(
        "<ram:ApplicableHeaderTradeDelivery><ram:ActualDeliverySupplyChainEvent><ram:OccurrenceDateTime><udt:DateTimeString format=\"102\">{}</udt:DateTimeString></ram:OccurrenceDateTime></ram:ActualDeliverySupplyChainEvent></ram:ApplicableHeaderTradeDelivery>\n",
        datum(&beleg.datum)
    ));

    // Abrechnung
    x.push_str("<ram:ApplicableHeaderTradeSettlement><ram:InvoiceCurrencyCode>EUR</ram:InvoiceCurrencyCode>");
    if let Some(bank) = firma.banken.iter().find(|b| !b.iban.trim().is_empty()) {
        x.push_str(&format!(
            "<ram:SpecifiedTradeSettlementPaymentMeans><ram:TypeCode>58</ram:TypeCode><ram:PayeePartyCreditorFinancialAccount><ram:IBANID>{}</ram:IBANID></ram:PayeePartyCreditorFinancialAccount>{}</ram:SpecifiedTradeSettlementPaymentMeans>",
            esc(&bank.iban.replace(' ', "")),
            if bank.bic.trim().is_empty() { String::new() } else { format!("<ram:PayeeSpecifiedCreditorFinancialInstitution><ram:BICID>{}</ram:BICID></ram:PayeeSpecifiedCreditorFinancialInstitution>", esc(bank.bic.trim())) }
        ));
    }
    let befreiung = if beleg.netto_rechnung { "<ram:ExemptionReason>Steuerschuldnerschaft des Leistungsempfängers</ram:ExemptionReason>" } else { "" };
    x.push_str(&format!(
        "<ram:ApplicableTradeTax><ram:CalculatedAmount>{}</ram:CalculatedAmount><ram:TypeCode>VAT</ram:TypeCode>{befreiung}<ram:BasisAmount>{}</ram:BasisAmount><ram:CategoryCode>{kategorie}</ram:CategoryCode><ram:RateApplicablePercent>{}</ram:RateApplicablePercent></ram:ApplicableTradeTax>",
        b(s.mwst), b(s.netto), b(satz)
    ));
    let kategorie_steuer = format!(
        "<ram:CategoryTradeTax><ram:TypeCode>VAT</ram:TypeCode><ram:CategoryCode>{kategorie}</ram:CategoryCode><ram:RateApplicablePercent>{}</ram:RateApplicablePercent></ram:CategoryTradeTax>",
        b(satz)
    );
    let mut abschlaege = Decimal::ZERO;
    let mut zuschlaege = Decimal::ZERO;
    let mut zu_ab = |betrag: Decimal, grund: &str, x: &mut String| {
        if betrag.is_zero() {
            return;
        }
        let zuschlag = betrag > Decimal::ZERO;
        if zuschlag { zuschlaege += betrag } else { abschlaege -= betrag }
        x.push_str(&format!(
            "<ram:SpecifiedTradeAllowanceCharge><ram:ChargeIndicator><udt:Indicator>{zuschlag}</udt:Indicator></ram:ChargeIndicator><ram:ActualAmount>{}</ram:ActualAmount><ram:Reason>{}</ram:Reason>{}</ram:SpecifiedTradeAllowanceCharge>",
            b(betrag.abs()),
            esc(grund),
            kategorie_steuer
        ));
    };
    zu_ab(-s.rabatt, &beleg.rabatt_text, &mut x);
    for z in &s.zuschlaege_netto {
        zu_ab(z.betrag, &z.text, &mut x);
    }
    let faellig = crate::journal::faellig_am(&beleg.datum, beleg.zahlungsziel_tage);
    let mut bedingung = String::new();
    if !beleg.skonto_prozent.is_zero() && beleg.skonto_tage > 0 {
        // Skonto im Format der XRechnung (#SKONTO#TAGE=…#PROZENT=…#)
        bedingung.push_str(&format!("#SKONTO#TAGE={}#PROZENT={}#\n", beleg.skonto_tage, b(beleg.skonto_prozent)));
    }
    x.push_str("<ram:SpecifiedTradePaymentTerms>");
    let beschreibung = if bedingung.is_empty() {
        if beleg.zahlungsziel_tage > 0 { format!("Zahlbar innerhalb von {} Tagen ohne Abzug", beleg.zahlungsziel_tage) } else { "Zahlbar sofort ohne Abzug".into() }
    } else {
        bedingung
    };
    x.push_str(&format!("<ram:Description>{}</ram:Description>", esc(&beschreibung)));
    if !faellig.is_empty() && s.zahlbetrag > Decimal::ZERO {
        x.push_str(&format!("<ram:DueDateDateTime><udt:DateTimeString format=\"102\">{}</udt:DateTimeString></ram:DueDateDateTime>", datum(&faellig)));
    }
    x.push_str("</ram:SpecifiedTradePaymentTerms>");
    x.push_str(&format!(
        "<ram:SpecifiedTradeSettlementHeaderMonetarySummation><ram:LineTotalAmount>{}</ram:LineTotalAmount><ram:ChargeTotalAmount>{}</ram:ChargeTotalAmount><ram:AllowanceTotalAmount>{}</ram:AllowanceTotalAmount><ram:TaxBasisTotalAmount>{}</ram:TaxBasisTotalAmount><ram:TaxTotalAmount currencyID=\"EUR\">{}</ram:TaxTotalAmount><ram:GrandTotalAmount>{}</ram:GrandTotalAmount><ram:TotalPrepaidAmount>{}</ram:TotalPrepaidAmount><ram:DuePayableAmount>{}</ram:DuePayableAmount></ram:SpecifiedTradeSettlementHeaderMonetarySummation>",
        b(s.positionen), b(zuschlaege), b(abschlaege), b(s.netto), b(s.mwst), b(s.brutto), b(s.abzug), b(s.zahlbetrag)
    ));
    for a in &v.abzuege {
        if !a.nummer.is_empty() {
            x.push_str(&format!(
                "<ram:InvoiceReferencedDocument><ram:IssuerAssignedID>{}</ram:IssuerAssignedID>{}</ram:InvoiceReferencedDocument>",
                esc(&a.nummer),
                if a.datum.is_empty() { String::new() } else { format!("<ram:FormattedIssueDateTime><qdt:DateTimeString format=\"102\">{}</qdt:DateTimeString></ram:FormattedIssueDateTime>", datum(&a.datum)) }
            ));
        }
    }
    x.push_str("</ram:ApplicableHeaderTradeSettlement>\n</rsm:SupplyChainTradeTransaction>\n</rsm:CrossIndustryInvoice>\n");
    x
}

const FX_XMP: &str = r#"<rdf:Description rdf:about="" xmlns:fx="urn:factur-x:pdfa:CrossIndustryDocument:invoice:1p0#">
<fx:DocumentType>INVOICE</fx:DocumentType>
<fx:DocumentFileName>factur-x.xml</fx:DocumentFileName>
<fx:Version>1.0</fx:Version>
<fx:ConformanceLevel>EN 16931</fx:ConformanceLevel>
</rdf:Description>
<rdf:Description rdf:about="" xmlns:pdfaExtension="http://www.aiim.org/pdfa/ns/extension/" xmlns:pdfaSchema="http://www.aiim.org/pdfa/ns/schema#" xmlns:pdfaProperty="http://www.aiim.org/pdfa/ns/property#">
<pdfaExtension:schemas><rdf:Bag><rdf:li rdf:parseType="Resource">
<pdfaSchema:schema>Factur-X PDFA Extension Schema</pdfaSchema:schema>
<pdfaSchema:namespaceURI>urn:factur-x:pdfa:CrossIndustryDocument:invoice:1p0#</pdfaSchema:namespaceURI>
<pdfaSchema:prefix>fx</pdfaSchema:prefix>
<pdfaSchema:property><rdf:Seq>
<rdf:li rdf:parseType="Resource"><pdfaProperty:name>DocumentFileName</pdfaProperty:name><pdfaProperty:valueType>Text</pdfaProperty:valueType><pdfaProperty:category>external</pdfaProperty:category><pdfaProperty:description>name of the embedded XML invoice file</pdfaProperty:description></rdf:li>
<rdf:li rdf:parseType="Resource"><pdfaProperty:name>DocumentType</pdfaProperty:name><pdfaProperty:valueType>Text</pdfaProperty:valueType><pdfaProperty:category>external</pdfaProperty:category><pdfaProperty:description>INVOICE</pdfaProperty:description></rdf:li>
<rdf:li rdf:parseType="Resource"><pdfaProperty:name>Version</pdfaProperty:name><pdfaProperty:valueType>Text</pdfaProperty:valueType><pdfaProperty:category>external</pdfaProperty:category><pdfaProperty:description>The actual version of the Factur-X XML schema</pdfaProperty:description></rdf:li>
<rdf:li rdf:parseType="Resource"><pdfaProperty:name>ConformanceLevel</pdfaProperty:name><pdfaProperty:valueType>Text</pdfaProperty:valueType><pdfaProperty:category>external</pdfaProperty:category><pdfaProperty:description>The conformance level of the embedded Factur-X data</pdfaProperty:description></rdf:li>
</rdf:Seq></pdfaSchema:property>
</rdf:li></rdf:Bag></pdfaExtension:schemas>
</rdf:Description>
"#;

/// Ergänzt die XMP-Metadaten des (von Typst als PDF/A-3b erzeugten) PDFs um die Factur-X-Angaben.
pub fn xmp_ergaenzen(pdf: Vec<u8>) -> Result<Vec<u8>, String> {
    let mut doc = lopdf::Document::load_mem(&pdf).map_err(|e| e.to_string())?;
    let katalog = doc.catalog().map_err(|e| e.to_string())?;
    let meta_ref = katalog.get(b"Metadata").and_then(|m| m.as_reference()).map_err(|e| format!("PDF ohne XMP: {e}"))?;
    let stream = doc.get_object_mut(meta_ref).and_then(|o| o.as_stream_mut()).map_err(|e| e.to_string())?;
    let inhalt = stream.decompressed_content().unwrap_or_else(|_| stream.content.clone());
    let text = String::from_utf8(inhalt).map_err(|e| e.to_string())?;
    let Some(pos) = text.rfind("</rdf:RDF>") else { return Err("XMP ohne rdf:RDF".into()) };
    let neu = format!("{}{}{}", &text[..pos], FX_XMP, &text[pos..]);
    stream.dict.remove(b"Filter");
    stream.dict.remove(b"DecodeParms");
    stream.set_content(neu.into_bytes());
    let mut out = vec![];
    doc.save_to(&mut out).map_err(|e| e.to_string())?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modelle::{Beleg, Kunde, Position};

    #[test]
    fn xml_enthaelt_summen_und_positionen() {
        let mut v = BelegVoll {
            beleg: Beleg { art: "rechnung".into(), nummer: "8298".into(), datum: "2024-06-21".into(), mwst_satz: Decimal::from(19), waehrung: "EUR".into(), zahlungsziel_tage: 14, ..Default::default() },
            kunde: Some(Kunde { name1: "Manfred Glinka".into(), strasse: "Hölderlinstr. 3".into(), plz: "75387".into(), ort: "Neubulach".into(), ..Default::default() }),
            ..Default::default()
        };
        let mut p = Position { art: "N".into(), text: "Gerüst <inkl.>\nAuf- und Abbau".into(), menge: Decimal::from(265), einheit: "m²".into(), ep: Decimal::new(885, 2), ..Default::default() };
        p.gp = gp(&p);
        v.positionen.push(p);
        v.summen = crate::rechnen::summen(&v.beleg, &v.positionen, &[], &[]);
        let firma = Firma { name: "Malergeschäft Seydt".into(), ort: "Neubulach".into(), steuernummer: "28 45245/32392".into(), ..Default::default() };
        assert!(moeglich(&v, &firma).is_ok());
        let x = xml(&v, &firma);
        assert!(x.contains("<ram:LineTotalAmount>2345.25</ram:LineTotalAmount>"));
        assert!(x.contains("<ram:TaxTotalAmount currencyID=\"EUR\">445.60</ram:TaxTotalAmount>"));
        assert!(x.contains("<ram:DuePayableAmount>2790.85</ram:DuePayableAmount>"));
        assert!(x.contains("unitCode=\"MTK\""));
        assert!(x.contains("Gerüst &lt;inkl.&gt;"));
        assert!(x.contains("<udt:DateTimeString format=\"102\">20240705</udt:DateTimeString>"));
        assert_eq!(einheit("Std."), "HUR");
    }
}
