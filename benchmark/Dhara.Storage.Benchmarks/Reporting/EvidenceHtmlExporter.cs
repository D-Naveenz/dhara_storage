using System.Text;
using System.Text.Encodings.Web;
using BenchmarkDotNet.Exporters;
using BenchmarkDotNet.Loggers;
using BenchmarkDotNet.Reports;

namespace Dhara.Storage.Benchmarks.Reporting;

/// <summary>
/// Writes a self-contained human evidence report into BDN's results directory,
/// session-stamped like the BDN log (e.g. <c>…-20260807-145440-report.html</c>).
/// </summary>
internal sealed class EvidenceHtmlExporter : IExporter
{
    public string Name => nameof(EvidenceHtmlExporter);

    public void ExportToLog(Summary summary, ILogger logger)
    {
        // File export is the primary surface; console stays for BDN progress.
    }

    public IEnumerable<string> ExportToFiles(Summary summary, ILogger consoleLogger)
    {
        Directory.CreateDirectory(summary.ResultsDirectoryPath);
        var path = EvidenceModel.SessionHtmlPath(summary);
        var html = BuildHtml(summary, Path.GetFileName(path), Path.GetFileName(EvidenceModel.SessionJsonPath(summary)));
        File.WriteAllText(path, html, Encoding.UTF8);
        consoleLogger.WriteLineInfo($"Evidence HTML: {path}");
        yield return path;
    }

    private static string BuildHtml(Summary summary, string htmlFileName, string jsonFileName)
    {
        var comparisons = EvidenceModel.BuildComparisons(summary);
        var candidateOnly = EvidenceModel.BuildCandidateOnly(summary);
        var allMethods = EvidenceModel.BuildAllMethods(summary);
        var host = EvidenceModel.ReadHost(summary.HostEnvironmentInfo);
        var generated = DateTime.UtcNow.ToString("u");

        var sb = new StringBuilder(16_384);
        sb.AppendLine("<!DOCTYPE html>");
        sb.AppendLine("<html lang=\"en\">");
        sb.AppendLine("<head>");
        sb.AppendLine("<meta charset=\"utf-8\" />");
        sb.AppendLine("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\" />");
        sb.AppendLine("<title>Dhara Storage — Binding Benchmarks</title>");
        sb.AppendLine("<style>");
        sb.AppendLine(Css);
        sb.AppendLine("</style>");
        sb.AppendLine("</head>");
        sb.AppendLine("<body>");
        sb.AppendLine("<div class=\"page\">");

        // 1) Header
        sb.AppendLine("<header class=\"hero\">");
        sb.AppendLine("<p class=\"eyebrow\">Dhara Storage</p>");
        sb.AppendLine("<h1>Binding Benchmark Report</h1>");
        sb.AppendLine("<p class=\"lede\">Rung 1 evidence: product path <strong>dhara-sd</strong> (B2) versus legacy in-process FFI (B1). Used to justify dropping the C ABI from the NuGet package.</p>");
        sb.AppendLine("<div class=\"meta-chips\">");
        AppendChip(sb, "📅", generated + " UTC");
        AppendChip(sb, "📦", Escape(summary.Title));
        AppendChip(sb, "⚙️", "BenchmarkDotNet " + Escape(host.BenchmarkDotNet));
        AppendChip(sb, "🏷️", "daemon-vs-ffi");
        sb.AppendLine("</div>");
        sb.AppendLine("</header>");

        // 2) How we measure
        sb.AppendLine("<section>");
        sb.AppendLine("<h2>📏 How we measure</h2>");
        sb.AppendLine("<div class=\"grid two\">");
        sb.AppendLine("<div class=\"card\">");
        sb.AppendLine("<h3>Baselines</h3>");
        sb.AppendLine("<ul>");
        sb.AppendLine("<li><strong>B1</strong> — C# harness → <code>dharastorage</code> FFI cdylib (in-process).</li>");
        sb.AppendLine("<li><strong>B2</strong> — C# → gRPC named pipe → <code>dhara-sd</code> (handle duplication / control RPCs).</li>");
        sb.AppendLine("</ul>");
        sb.AppendLine("<p class=\"note\">Means are wall time per operation from BenchmarkDotNet. Ratios are <em>B2 mean ÷ B1 mean</em> (lower is better for B2). Thresholds live in <code>docs/binding-benchmarks.md</code>.</p>");
        sb.AppendLine("</div>");
        sb.AppendLine("<div class=\"card\">");
        sb.AppendLine("<h3>Host environment</h3>");
        sb.AppendLine("<dl class=\"specs\">");
        AppendSpec(sb, "OS", host.Os);
        AppendSpec(sb, "CPU", host.Processor);
        if (host.Cores is not ("—" or ""))
        {
            AppendSpec(sb, "Cores", host.Cores);
        }

        AppendSpec(sb, "Runtime", host.Runtime);
        AppendSpec(sb, ".NET SDK", host.DotNetSdk);
        AppendSpec(sb, "Architecture", host.Architecture);
        if (summary.BenchmarksCases.Length > 0)
        {
            AppendSpec(sb, "Job", EvidenceModel.FormatJobSummary(summary.BenchmarksCases[0]));
        }

        sb.AppendLine("</dl>");
        sb.AppendLine("</div>");
        sb.AppendLine("</div>");
        sb.AppendLine("</section>");

        // 3) Results
        sb.AppendLine("<section>");
        sb.AppendLine("<h2>📊 Results</h2>");

        sb.AppendLine("<h3>Verdict guide</h3>");
        sb.AppendLine("<ul class=\"legend\">");
        sb.AppendLine("<li><span class=\"badge pass\">✅ Pass</span> Candidate mean is within the scenario budget vs B1 (see <code>docs/binding-benchmarks.md</code>).</li>");
        sb.AppendLine("<li><span class=\"badge watch\">⚠️ Watch</span> Over budget, but within 1.5× the allowed overhead — worth a closer look.</li>");
        sb.AppendLine("<li><span class=\"badge fail\">❌ Fail</span> Clearly over the decision threshold for this scenario.</li>");
        sb.AppendLine("<li><span class=\"badge informational\">ℹ️ Informational</span> No pass/fail budget configured; ratio shown for context only.</li>");
        sb.AppendLine("<li><span class=\"badge missing\">❔ Missing</span> Baseline or candidate was not in this run (filter / smoke suite).</li>");
        sb.AppendLine("</ul>");

        sb.AppendLine("<h3>Findings</h3>");
        sb.AppendLine("<ul class=\"findings\">");
        foreach (var row in comparisons)
        {
            var emoji = VerdictEmoji(row.Verdict);
            var label = row.Verdict.ToString();
            var detail = row.Verdict == ScenarioVerdict.Missing
                ? "baseline or candidate missing from this run"
                : $"ratio {EvidenceModel.FormatRatio(row.Ratio)}"
                  + (row.Pair.MaxMeanOverhead is { } max
                      ? $" (budget ≤ +{max:P0})"
                      : " (no budget)");
            sb.AppendLine("<li class=\"finding\">");
            sb.AppendLine($"<span class=\"badge {row.Verdict.ToString().ToLowerInvariant()}\">{emoji} {label}</span>");
            sb.AppendLine($"<span class=\"finding-body\"><strong>{Escape(row.Pair.DisplayName)}</strong><span class=\"finding-detail\">{Escape(detail)}</span></span>");
            sb.AppendLine("</li>");
        }

        sb.AppendLine("</ul>");

        sb.AppendLine("<h3>Comparison (B1 vs B2)</h3>");
        sb.AppendLine("<div class=\"table-wrap\">");
        sb.AppendLine("<table>");
        sb.AppendLine("<thead><tr>");
        sb.AppendLine("<th class=\"text\">Scenario</th>");
        sb.AppendLine("<th class=\"num\">Baseline (B1)</th>");
        sb.AppendLine("<th class=\"num\">Candidate (B2)</th>");
        sb.AppendLine("<th class=\"num\">Ratio</th>");
        sb.AppendLine("<th class=\"num\">Alloc. change</th>");
        sb.AppendLine("<th class=\"verdict\">Verdict</th>");
        sb.AppendLine("</tr></thead>");
        sb.AppendLine("<tbody>");
        foreach (var row in comparisons)
        {
            var b1 = row.Baseline is null ? "—" : EvidenceModel.FormatTime(row.Baseline.MeanNs);
            var b2 = row.Candidate is null ? "—" : EvidenceModel.FormatTime(row.Candidate.MeanNs);
            var delta = row.Baseline is null || row.Candidate is null
                ? "—"
                : EvidenceModel.FormatSignedBytes(row.AllocatedDelta);
            sb.AppendLine("<tr>");
            sb.AppendLine($"<td class=\"text\">{Escape(row.Pair.DisplayName)}</td>");
            sb.AppendLine($"<td class=\"num\">{Escape(b1)}</td>");
            sb.AppendLine($"<td class=\"num\">{Escape(b2)}</td>");
            sb.AppendLine($"<td class=\"num\">{Escape(EvidenceModel.FormatRatio(row.Ratio))}</td>");
            sb.AppendLine($"<td class=\"num\">{Escape(delta)}</td>");
            sb.AppendLine($"<td class=\"verdict\"><span class=\"badge {row.Verdict.ToString().ToLowerInvariant()}\">{VerdictEmoji(row.Verdict)} {row.Verdict}</span></td>");
            sb.AppendLine("</tr>");
        }

        sb.AppendLine("</tbody></table>");
        sb.AppendLine("</div>");
        sb.AppendLine("<p class=\"note\">Alloc. change is managed memory B2 − B1 per operation (positive means B2 allocated more).</p>");

        if (candidateOnly.Count > 0)
        {
            sb.AppendLine("<h3>Candidate-only (no FFI peer)</h3>");
            sb.AppendLine("<div class=\"table-wrap\">");
            sb.AppendLine("<table>");
            sb.AppendLine("<thead><tr><th class=\"text\">Method</th><th class=\"num\">Mean</th><th class=\"num\">Error</th><th class=\"num\">StdDev</th><th class=\"num\">Allocated</th></tr></thead>");
            sb.AppendLine("<tbody>");
            foreach (var m in candidateOnly)
            {
                AppendDetailRow(sb, m);
            }

            sb.AppendLine("</tbody></table>");
            sb.AppendLine("</div>");
        }

        sb.AppendLine("<h3>Detailed results</h3>");
        sb.AppendLine("<p class=\"note\">Same run as above — curated columns for drill-down.</p>");
        sb.AppendLine("<div class=\"table-wrap\">");
        sb.AppendLine("<table>");
        sb.AppendLine("<thead><tr><th class=\"text\">Method</th><th class=\"num\">Mean</th><th class=\"num\">Error</th><th class=\"num\">StdDev</th><th class=\"num\">Allocated</th></tr></thead>");
        sb.AppendLine("<tbody>");
        foreach (var m in allMethods)
        {
            AppendDetailRow(sb, m);
        }

        sb.AppendLine("</tbody></table>");
        sb.AppendLine("</div>");
        sb.AppendLine("</section>");

        // 4) Footer
        sb.AppendLine("<footer>");
        sb.AppendLine("<p>🛠 Manual harness — not part of CI/CD. Numbers are machine-specific; re-run after transport or packaging changes.</p>");
        sb.AppendLine($"<p>Artifacts: <code>{Escape(htmlFileName)}</code> (this file) · <code>{Escape(jsonFileName)}</code> (machine). Ladder &amp; thresholds: <code>docs/binding-benchmarks.md</code>.</p>");
        sb.AppendLine("<p class=\"muted\">Future user-facing evidence will compare the daemon path to a C# approximation of Dhara features — not bare BCL one-liners.</p>");
        sb.AppendLine("</footer>");

        sb.AppendLine("</div>");
        sb.AppendLine("</body></html>");
        return sb.ToString();
    }

    private static void AppendDetailRow(StringBuilder sb, MethodStats m)
    {
        sb.AppendLine("<tr>");
        sb.AppendLine($"<td class=\"text\">{Escape(m.Title)}</td>");
        sb.AppendLine($"<td class=\"num\">{Escape(EvidenceModel.FormatTime(m.MeanNs))}</td>");
        sb.AppendLine($"<td class=\"num\">{Escape(EvidenceModel.FormatTime(m.ErrorNs))}</td>");
        sb.AppendLine($"<td class=\"num\">{Escape(EvidenceModel.FormatTime(m.StdDevNs))}</td>");
        sb.AppendLine($"<td class=\"num\">{Escape(EvidenceModel.FormatBytes(m.AllocatedBytes))}</td>");
        sb.AppendLine("</tr>");
    }

    private static void AppendChip(StringBuilder sb, string emoji, string text) =>
        sb.AppendLine($"<span class=\"chip\">{emoji} {Escape(text)}</span>");

    private static void AppendSpec(StringBuilder sb, string label, string value) =>
        sb.AppendLine($"<dt>{Escape(label)}</dt><dd>{Escape(value)}</dd>");

    private static string VerdictEmoji(ScenarioVerdict verdict) =>
        verdict switch
        {
            ScenarioVerdict.Pass => "✅",
            ScenarioVerdict.Watch => "⚠️",
            ScenarioVerdict.Fail => "❌",
            ScenarioVerdict.Informational => "ℹ️",
            ScenarioVerdict.Missing => "❔",
            _ => "•",
        };

    private static string Escape(string? value) =>
        HtmlEncoder.Default.Encode(value ?? string.Empty);

    private const string Css = """
:root {
  --bg: #f4f6f8;
  --surface: #ffffff;
  --ink: #1a2332;
  --muted: #5b6b7c;
  --line: #d8dee6;
  --accent: #0b6e4f;
  --accent-soft: #e6f4ef;
  --pass: #0b6e4f;
  --pass-bg: #e6f4ef;
  --watch: #9a6700;
  --watch-bg: #fff6dd;
  --fail: #b42318;
  --fail-bg: #fdecea;
  --info: #175cd3;
  --info-bg: #eff4ff;
  --missing: #667085;
  --missing-bg: #f2f4f7;
  --shadow: 0 10px 30px rgba(26, 35, 50, 0.08);
  --radius: 14px;
  --pad: 1.1rem;
  font-family: "Segoe UI", "Helvetica Neue", ui-sans-serif, system-ui, sans-serif;
}
* { box-sizing: border-box; }
body {
  margin: 0;
  color: var(--ink);
  background:
    radial-gradient(1200px 400px at 10% -10%, #d9f2e8 0%, transparent 55%),
    radial-gradient(900px 360px at 100% 0%, #dbe7ff 0%, transparent 50%),
    var(--bg);
  line-height: 1.5;
}
.page { max-width: 1100px; margin: 0 auto; padding: 2rem 1.25rem 3rem; }
.hero {
  background: linear-gradient(135deg, #0b6e4f 0%, #0f766e 45%, #1d4ed8 120%);
  color: #fff;
  border-radius: calc(var(--radius) + 4px);
  padding: var(--pad);
  box-shadow: var(--shadow);
  margin-bottom: 1.75rem;
}
.eyebrow {
  margin: 0 0 0.35rem;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  font-size: 0.75rem;
  opacity: 0.85;
  font-weight: 600;
}
.hero h1 { margin: 0 0 0.65rem; font-size: 1.9rem; line-height: 1.2; }
.lede { margin: 0 0 1rem; max-width: 46rem; opacity: 0.95; }
.lede strong { color: #ecfdf5; }
.meta-chips { display: flex; flex-wrap: wrap; gap: 0.5rem; }
.chip {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  background: rgba(255,255,255,0.16);
  border: 1px solid rgba(255,255,255,0.22);
  border-radius: 999px;
  padding: 0.28rem 0.7rem;
  font-size: 0.82rem;
  max-width: 100%;
  overflow-wrap: anywhere;
}
section { margin-bottom: 1.75rem; }
h2 { margin: 0 0 0.85rem; font-size: 1.35rem; }
h3 { margin: 1.25rem 0 0.65rem; font-size: 1.05rem; }
.grid.two {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 1rem;
  align-items: stretch;
}
@media (max-width: 800px) {
  .grid.two { grid-template-columns: 1fr; }
}
.card {
  background: var(--surface);
  border: 1px solid var(--line);
  border-radius: var(--radius);
  padding: var(--pad);
  box-shadow: var(--shadow);
  min-width: 0;
  overflow: hidden;
}
.card > h3:first-child { margin: 0 0 0.75rem; }
.card ul { margin: 0 0 0.75rem; padding-left: 1.15rem; }
.card .note { margin: 0; }
.note { color: var(--muted); font-size: 0.92rem; margin: 0.55rem 0 0; }
.specs {
  display: grid;
  grid-template-columns: 7.5rem minmax(0, 1fr);
  gap: 0.45rem 0.85rem;
  margin: 0;
  align-items: start;
}
.specs dt { color: var(--muted); font-weight: 600; padding-top: 0.1rem; }
.specs dd {
  margin: 0;
  min-width: 0;
  overflow-wrap: anywhere;
  word-break: break-word;
}
.legend {
  list-style: none;
  padding: 0;
  margin: 0 0 1rem;
  display: grid;
  gap: 0.5rem;
}
.legend li {
  display: flex;
  align-items: flex-start;
  gap: 0.75rem;
  background: var(--surface);
  border: 1px solid var(--line);
  border-radius: 10px;
  padding: 0.75rem var(--pad);
  font-size: 0.92rem;
  color: var(--muted);
}
.legend .badge { flex: 0 0 auto; margin-top: 0.05rem; }
.findings { list-style: none; padding: 0; margin: 0; display: grid; gap: 0.45rem; }
.finding {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  background: var(--surface);
  border: 1px solid var(--line);
  border-radius: 10px;
  padding: 0.75rem var(--pad);
  min-width: 0;
}
.finding-body {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: 0.25rem 0.55rem;
  min-width: 0;
}
.finding-detail { color: var(--muted); font-weight: 400; }
.table-wrap {
  overflow-x: auto;
  background: var(--surface);
  border: 1px solid var(--line);
  border-radius: var(--radius);
  box-shadow: var(--shadow);
}
table {
  width: 100%;
  border-collapse: collapse;
  font-size: 0.92rem;
  margin: 0;
  table-layout: auto;
}
th, td {
  padding: 0.7rem 1rem;
  border-bottom: 1px solid var(--line);
  vertical-align: middle;
}
th {
  background: var(--accent-soft);
  color: var(--accent);
  font-size: 0.78rem;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  white-space: nowrap;
}
th.text, td.text { text-align: left; }
th.num, td.num {
  font-variant-numeric: tabular-nums;
  text-align: right;
  white-space: nowrap;
}
th.verdict, td.verdict { text-align: left; white-space: nowrap; }
tbody tr:nth-child(even) { background: #fafbfc; }
tbody tr:hover { background: #eef8f3; }
tbody tr:last-child td { border-bottom: none; }
.badge {
  display: inline-flex;
  align-items: center;
  gap: 0.25rem;
  border-radius: 999px;
  padding: 0.2rem 0.6rem;
  font-size: 0.78rem;
  font-weight: 700;
  line-height: 1.2;
  white-space: nowrap;
}
.badge.pass { color: var(--pass); background: var(--pass-bg); }
.badge.watch { color: var(--watch); background: var(--watch-bg); }
.badge.fail { color: var(--fail); background: var(--fail-bg); }
.badge.informational { color: var(--info); background: var(--info-bg); }
.badge.missing { color: var(--missing); background: var(--missing-bg); }
footer {
  margin-top: 2rem;
  padding-top: 1rem;
  border-top: 1px dashed var(--line);
  color: var(--muted);
  font-size: 0.9rem;
}
footer p { margin: 0.35rem 0; }
.muted { opacity: 0.9; }
code {
  font-family: ui-monospace, "Cascadia Code", Consolas, monospace;
  font-size: 0.88em;
  background: #eef2f6;
  padding: 0.05rem 0.3rem;
  border-radius: 4px;
}
.hero code { background: rgba(255,255,255,0.18); color: #fff; }
""";
}
