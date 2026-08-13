import { describe, it, expect } from "vitest";
import { kbStr, fullRegPath, mdCell, oneLine } from "@/composables/scan/scanExportHelpers";

describe("kbStr", () => {
  it("valeur < 1024 → affiche KB", () => {
    expect(kbStr(512)).toBe("512 KB");
    expect(kbStr(0)).toBe("0 KB");
    expect(kbStr(1023)).toBe("1023 KB");
  });

  it("valeur >= 1024 → affiche MB (arrondi)", () => {
    expect(kbStr(1024)).toBe("1 MB");
    expect(kbStr(2048)).toBe("2 MB");
    expect(kbStr(1500)).toBe("1 MB");
    expect(kbStr(1536)).toBe("2 MB");
  });

  it("grandes valeurs converties en MB", () => {
    expect(kbStr(10240)).toBe("10 MB");
  });
});

describe("fullRegPath", () => {
  it("expanse HKCU avec sous-chemin", () => {
    expect(fullRegPath("HKCU\\Software\\Microsoft")).toBe("HKEY_CURRENT_USER\\Software\\Microsoft");
  });

  it("expanse HKCU seul", () => {
    expect(fullRegPath("HKCU")).toBe("HKEY_CURRENT_USER");
  });

  it("expanse HKLM avec sous-chemin", () => {
    expect(fullRegPath("HKLM\\SYSTEM\\CurrentControlSet")).toBe("HKEY_LOCAL_MACHINE\\SYSTEM\\CurrentControlSet");
  });

  it("expanse HKCR", () => {
    expect(fullRegPath("HKCR\\.exe")).toBe("HKEY_CLASSES_ROOT\\.exe");
  });

  it("expanse HKU", () => {
    expect(fullRegPath("HKU\\S-1-5-21")).toBe("HKEY_USERS\\S-1-5-21");
  });

  it("ajoute le nom si fourni (chemin sans barre oblique finale)", () => {
    expect(fullRegPath("HKCU\\Software", "Run")).toBe("HKEY_CURRENT_USER\\Software\\Run");
  });

  it("n'ajoute pas de double backslash si chemin se termine par \\", () => {
    expect(fullRegPath("HKCU\\Software\\", "Run")).toBe("HKEY_CURRENT_USER\\Software\\Run");
  });

  it("chemin non-HKCU/HKLM est retourné tel quel", () => {
    expect(fullRegPath("UNKNOWN\\Path")).toBe("UNKNOWN\\Path");
  });

  it("sans nom — retourne seulement le chemin expansé", () => {
    expect(fullRegPath("HKLM\\SOFTWARE")).toBe("HKEY_LOCAL_MACHINE\\SOFTWARE");
  });
});

describe("mdCell", () => {
  it("échappe le pipe (délimiteur de colonne de tableau)", () => {
    expect(mdCell("cmd.exe /c dir | more")).toBe("cmd.exe /c dir \\| more");
  });

  it("échappe le backslash en premier pour ne pas doubler le \\| inséré ensuite", () => {
    expect(mdCell("a\\|b")).toBe("a\\\\\\|b");
  });

  it("remplace les sauts de ligne par un espace", () => {
    expect(mdCell("line1\nline2")).toBe("line1 line2");
    expect(mdCell("line1\r\nline2")).toBe("line1 line2");
  });

  it("remplace le backtick par une apostrophe (ne casse pas un span de code)", () => {
    expect(mdCell("`rm -rf /`")).toBe("'rm -rf /'");
  });

  it("échappe les chevrons pour empêcher le HTML brut de survivre au rendu", () => {
    expect(mdCell("<img src=x onerror=alert(1)>")).toBe("&lt;img src=x onerror=alert(1)&gt;");
  });

  it("null/undefined → chaîne vide", () => {
    expect(mdCell(null)).toBe("");
    expect(mdCell(undefined)).toBe("");
  });

  it("valeur repro cycle 159 : erreur de scan avec backtick et pipe ne casse plus le span de code", () => {
    // Reproduit exactement le bug corrigé dans useScanExportMd.ts : la variable
    // de boucle "e" masquait la fonction d'échappement "e", laissant ce genre
    // de valeur passer intégralement non échappée dans le rapport exporté.
    const raw = "exception in `Get-WmiObject` call | pipe-and-backtick";
    const escaped = mdCell(raw);
    expect(escaped).not.toContain("`Get-WmiObject`");
    expect(escaped).not.toContain(" | pipe");
    expect(escaped).toBe("exception in 'Get-WmiObject' call \\| pipe-and-backtick");
  });
});

describe("oneLine", () => {
  it("aplati les retours à la ligne (LF et CRLF) sur une seule ligne", () => {
    expect(oneLine("a\nb")).toBe("a b");
    expect(oneLine("a\r\nb")).toBe("a b");
    expect(oneLine("a\nb\r\nc")).toBe("a b c");
  });

  it("empêche un nom d'autorun malveillant d'usurper un en-tête de section TXT", () => {
    // Un \n embarqué dans un nom d'entité (registre/WMI accepte n'importe quel
    // Unicode) injecterait sinon une fausse ligne "=== FIN ===" dans le rapport.
    const evil = "Updater\n============  FIN DU RAPPORT  ============";
    expect(oneLine(evil)).not.toContain("\n");
    expect(oneLine(evil)).toBe("Updater ============  FIN DU RAPPORT  ============");
  });

  it("gère null/undefined sans lever", () => {
    expect(oneLine(null)).toBe("");
    expect(oneLine(undefined)).toBe("");
  });
});
