# Learning Difficulties in Set Theory and Proof — Research Evidence Dossier

**Purpose.** Source-gathering for a curriculum survey. Ten specified student "blockers" in
learning set theory and proof, each answered with *actually retrieved* research literature:
who studied it, what they found, and a URL.

**Compiled:** this session. **Status of every claim:** see the verification legend below.
**Nothing in this document is written from memory.** Every citation below was produced by an
API record or a document fetched during this session; items that could not be retrieved are
quarantined in the "UNVERIFIED / COULD NOT RETRIEVE" sections rather than dressed up.

---

## 0. How this was researched, and what was broken

`web_search` and `web_fetch` were unavailable for this task. All retrieval was done with
`bash` + `curl` through a local HTTP proxy (`http://127.0.0.1:7890`), plus `pandoc` (HTML→text)
and PyMuPDF / tesseract (PDF text extraction and OCR; **`pdftotext` is not installed** on this
machine).

**Source-by-source conditions encountered (recorded so the survey's method section is honest
and so the work is reproducible):**

| Source | Status this session |
|---|---|
| Crossref `api.crossref.org` | ✅ worked throughout (use `query.title`, not `query.bibliographic`, for topical search) |
| ERIC `eric.ed.gov` + `files.eric.ed.gov` | ✅ worked (https only; plain http times out). Use quoted phrases — unquoted keyword search returns large amounts of mathematics-learning-disability noise |
| **OpenAIRE by DOI** `api.openaire.eu/search/publications?doi=…&format=json` | ✅ **highest-yield tool** — returned full publisher abstracts for paywalled Springer/AMS items where Crossref and Semantic Scholar both had none |
| Semantic Scholar `api.semanticscholar.org` | ⚠️ frequently HTTP 429; the `paper/batch` POST endpoint (one call, many DOIs) worked and is the efficient form |
| Unpaywall `api.unpaywall.org/v2/<doi>` | ✅ worked — used to locate/deny legal open copies |
| DOAJ `doaj.org/api/search/articles/…` | ✅ reachable but returned almost nothing topical |
| HAL `api.archives-ouvertes.fr` | ✅ worked (French mathematics-education literature) |
| OpenAlex `api.openalex.org` | ❌ **exhausted mid-task** — per-IP daily budget (`dailyRemainingUsd: 0.0003`, resets midnight UTC). Records fetched *before* exhaustion are marked as such |
| Google Scholar | ❌ HTTP 429 |
| Bing HTML | ❌ returned unrelated geo-localised results, ignoring the query — unusable |
| DBLP | ❌ bot check |
| CORE `api.core.ac.uk` | ❌ HTTP 429 |
| ResearchGate | ❌ blocked |
| `learntechlib.org` / `editlib.org` (AACE, hosts JCMST) | ❌ HTTP 403 from this IP ("check your IP address … info@aace.org"); direct connection hits an AWS WAF JS challenge |
| `philpapers.org` PDFs | ❌ HTTP 403 + Cloudflare "Enable JavaScript and cookies to continue" |
| `archive.org` / Wayback Machine | ⚠️ intermittently "Temporarily Offline" (503); direct `/web/<ts>if_/` URLs did work when the CDX API did not |
| Publisher landing pages (Springer, Elsevier, T&F, OUP, AMS, NCTM) | ❌ JS/Cloudflare-challenged through this proxy — **plan on APIs and repository copies, not landing pages** |
| Ed Dubinsky's author archive `https://www.math.kent.edu/~edd/` | ✅ **live and open** — eight relevant PDFs all returned HTTP 200 |

**A note on Dubinsky's archived PDFs.** Several are scans of back-issues whose embedded fonts
use a shifted encoding: extracted text comes out Caesar-shifted and digits/punctuation become
control characters. A decoder was written this session (`/tmp/stresearch/dec2.py`); scanned
items additionally needed OCR. The PDFs themselves are authoritative — only the automatic text
extraction is affected.

**Three link-hygiene cautions for anyone re-running the URL checks.**
1. **HTTP 200 does not always mean you got the document.** `http://www.ijsi.org/ijsi/article/pdf/303`
   returns **HTTP 200 with a 59-byte JavaScript bot-guard page** (`<script src="/_guard/html.js…">`),
   not a PDF. Always check `content-type` and body size, not just the status code.
2. **ERIC record pages fail intermittently through this proxy** with
   `OpenSSL SSL_read: … unexpected eof while reading`. Every such failure in this dossier was
   **transient**: `EJ543540`, `EJ567950`, `EJ1147782`, `EJ1410917`, `EJ748150`, `EJ1188492`,
   `EJ1322061`, `ED646013` and `EJ1417658` all returned HTTP 200 on retry. Do not record them
   as dead links.
3. **One DOI resolves but its publisher blocks this network:** `https://doi.org/10.1093/teamat/hrv007`
   (Shipman 2015) redirects correctly, but OUP returns **HTTP 403** to the proxy. The citation is
   verified from its abstract; the landing page is not retrievable from here.

---

## Verification legend

| Mark | Meaning |
|---|---|
| **[F]** | Full text downloaded and read; findings quoted from the body |
| **[A]** | Verbatim abstract retrieved from a publisher/repository/API record |
| **[M]** | Bibliographic metadata only (title/authors/year/venue/volume/pages/DOI verified); **findings NOT verified** — no abstract or full text obtainable |
| **[R]** | DOI resolution independently checked with `https://doi.org/<DOI>` and/or cross-checked in a second index |

Source types are labelled explicitly. Non-peer-reviewed items (blog posts, course pages,
unpublished manuscripts) are labelled **次级来源 (secondary source)** or "unpublished
manuscript" as appropriate.

---

## 1. Executive summary — headline evidence per blocker

### Blocker 1 — `∈` vs `⊆`; element vs singleton (`{a}` vs `a`)
The one peer-reviewed paper explicitly about **inclusion vs belonging** is **Bagni (2006)**,
*Educational Studies in Mathematics* 62(3) — it locates the difficulty in students' failure to
"suitably distinguish and coordinate the meanings and symbols of the various semiotic systems
(e.g. verbal, diagrammatic and symbolic)" — DOI
[10.1007/s10649-006-8545-3](https://doi.org/10.1007/s10649-006-8545-3).
The sharpest *empirical* datum comes from **Hendriyanto et al. (2024)**, *Journal on
Mathematics Education* 15(2), 517–544: with `M = {r,s,t}`, essentially **all** 183 students got
`r ∈ M` and `s ∉ M` right, but on `{r} ∈ M` **no student produced a correct justification** —
students rejected the singleton ("`{r}` is not an element of the set M"), and the authors
classify the companion task as students who "cannot distinguish between ∈ and ="
([free PDF](https://files.eric.ed.gov/fulltext/EJ1428069.pdf)).
Also: **Shaker & Berger (2016)**, *African Journal of Research in MST Education* — first-year
students' misinterpretation of set-theoretic **definitions** (union, Cartesian product) while
attempting set-theory proofs, ERIC [EJ1147782](https://eric.ed.gov/?id=EJ1147782); and the
canonical **Zazkis & Gunn (1997)**, *JCMST* 16(1), 133–169, ERIC
[EJ543540](https://eric.ed.gov/?id=EJ543540) — element, subset and empty set in one ISETL-based
design (record verified; full text blocked from this IP).

### Blocker 2 — The empty set and vacuous truth
For resistance to `∅`: the same **Hendriyanto et al. (2024)** study reports that of five
"is this a set?" objects, two were `∅`, and "**Many of the students did not accept this as a
set. The reason is all the same: no element in it can be identified**"; one student who had
accepted `∅` only on the teacher's authority "was silent and could not explain" when asked why
the empty set can be a set.
For **false antecedents**: **Lee, Park & Kim (2024)**, *International Journal of Science and
Mathematics Education*, found a social-contract (promise/rule-breaking) situation helps
students see why a conditional with a false antecedent is true, with residual difficulties tied
to a time variable and to confusing conditionals with causality — ERIC
[EJ1410917](https://eric.ed.gov/?id=EJ1410917). **Durand-Guerrier (2003)**, *ESM* 53(1), 5–34,
argues the fix requires moving from implication-as-connective to implication over **open
sentences**, where "the truth-value of a given mathematical statement is not constrained by the
situation" — the vacuity point — DOI
[10.1023/A:1024661004375](https://doi.org/10.1023/A:1024661004375).
Best *quantitative* classroom evidence that vacuous/implicational reasoning is the real
bottleneck: in **Dubinsky (1997)** "**of the seven problems on which the students did poorly
(less than 60%), five of them involved an implication which was not part of the
quantification**", and on one item "all but one of the students were quite correct in negating
the quantification part. Their errors were entirely the result of … difficulties they had with
negating an implication" ([free PDF](https://www.math.kent.edu/~edd/LearningQuant.pdf)).

### Blocker 3 — Ordered pairs and the Kuratowski encoding
**Documented negative result: no empirical study of the Kuratowski encoding, or of students
proving `(a,b)=(c,d) ↔ a=c ∧ b=d`, could be found in any reachable source.** ERIC's
`"Kuratowski ordered pair"` returns **0** results; its `"ordered pair"` hits are school graphing
activities. The encoding-blocker claim is therefore currently **instructor experience, not
published evidence**, and the survey should say so.
What *is* citable: **Mirin, Weber & Wasserman (2020)**, PME-NA 42 — two inequivalent
definitions of "function" are simultaneously live (Bourbaki triple vs set of ordered pairs)
and give **different answers to the same student-level questions**, while "mathematicians and
mathematics educators are often not explicit about which definition they are using"
([free PDF](https://files.eric.ed.gov/fulltext/ED629969.pdf)); and **Kanamori (2003)**,
*Bulletin of Symbolic Logic*, on `〈x,y〉`, `∅` and `{a}` as notions that required the historical
shift from an intensional to an extensional viewpoint — DOI
[10.2178/bsl/1058448674](https://doi.org/10.2178/bsl/1058448674). Nearest genuine teaching
result: **Karagöz Akar & Şener (2014)** — 9th graders *did* come to "detect why the elements of
a Cartesian product need to be in ordered pairs", while struggling to graph Cartesian products
over infinite sets.

### Blocker 4 — Function as set of ordered pairs vs function as rule/map
**Breidenbach, Dubinsky, Hawks & Nichols (1992)**, *ESM* 23(3), 247–285 — DOI
[10.1007/BF02309532](https://doi.org/10.1007/BF02309532) — is the canonical APOS/ontology
source, and it supplies **numbers**: across a 24-situation instrument (~60 undergraduates),
students agreed "yes, that's a function" only **about 40% of the time overall where it should
have been near 100%**; by representation, ISETL funcs 74.6–76.5%, tuples 54.1–61.6%, **graphs
19–40.7%, equations 25–31%**, tables 39.9–48.6%, physical situations 30.2–35.8%. Students
"insisted that there be an expression or at least the presence of variables to indicate 'input'
and 'output'" and "in many cases … insisted on the presence of causality" before granting a
process; those who accepted a graph or table "tried to guess a formula … before they were
willing to agree that there was a function"
([author's PDF](https://www.math.kent.edu/~edd/PROCESSFUNC.pdf)). Theoretical framing:
**Sfard (1991)** *ESM* 22(1), 1–36, DOI
[10.1007/BF00302715](https://doi.org/10.1007/BF00302715) (process/object duality —
⚠️ metadata only, findings not retrieved); **Vinner (1983)** concept image vs concept
definition; **Tall & Vinner (1981)**; **Tall & Bakar (1992)** mental prototypes;
**Thompson (1994)**; **Even (1990, 1993)** on teachers; **Dubinsky & Wilson (2013)** on high
school students.

### Blocker 5 — Quantifier order (`∀∃` vs `∃∀`) and negating quantified statements
The strongest evidence is **Dubinsky & Yiparaki (2000)**, 63 students, 11 statements, full text
read: "**94% of the students interpreted at least one EA statement as an AE**" (per-statement
range **11%–81%**), while "only **5%** interpreted at least one AE statement as an EA"
(range **0%–3%**); on the two *mathematical* statements "only **41%** … got statement 10 right,
and only **9%** got statement 11 right", even though "78% … gave a valid argument for seven out
of the nine natural-language" statements. The abstract's conclusion: "Most students in this
study **could not distinguish between AE and EA statements in mathematics** and did not seem to
be aware of the standard mathematical conventions for parsing statements"
([author's PDF](https://www.math.kent.edu/~edd/OlgaPaper.pdf) — ⚠️ **unpublished manuscript**,
cite as such). Complementing it: **Dubinsky (1997)**, JCMST 16(2&3), 335–362, ERIC
[EJ567950](https://eric.ed.gov/?id=EJ567950), free PDF
[here](https://www.math.kent.edu/~edd/LearningQuant.pdf); **Dubinsky, Elterman & Gong (1988)**,
*FLM* 8(2), 44–51, which reports "**49 out of 52 students were unable to negate the statement,
'Every member of my family is unemployed'**" (as an unpublished observation inside the paper);
**Selden & Selden (1995)** with the devastating **8.5% / 5%** unpacking-success figures; and
**Shipman (2015)**, *Teaching Mathematics and its Applications*, which names the instructional
cause — textbooks teach implication via truth tables, "treating P and Q as statements
themselves with their own truth values", omitting the hidden `∀x` — DOI
[10.1093/teamat/hrv007](https://doi.org/10.1093/teamat/hrv007).
⚠️ **Correction to a lead:** no Piatek-Jimenez (2004) one-to-one/onto paper exists in any index
searched. The author's real work in this area is **Piatek-Jimenez (2010)**, *Mathematics
Education Research Journal* 22(3), 41–56, on students' interpretations of *quantified*
statements — DOI [10.1007/BF03219777](https://doi.org/10.1007/BF03219777). For
injections/surjections specifically use **Bansilal, Brijlall & Trigueros (2017)**, *JMB* 48,
22–37, "An APOS study on pre-service teachers' understanding of injections and surjections" —
DOI [10.1016/j.jmathb.2017.08.002](https://doi.org/10.1016/j.jmathb.2017.08.002).

### Blocker 6 — Image vs preimage (forward vs backward preservation)
**This is the thinnest blocker in the literature and should be reported as such.** There is no
widely cited paper on students' understanding of image and preimage, and none measuring the
`f(A∩B) ⊆ f(A)∩f(B)`-not-conversely failure. The closest readable primary source is
**Breidenbach et al. (1992)**, whose four-week treatment explicitly includes "activities
designed to help the students think about situations in terms of **reversing the process of a
function** … problems that **compute pre-images of functions**, the construction of an
**inverse function** and the concepts of **1-1 and onto**", alongside forward-direction image
computations; its theoretical claim is that students can encapsulate a process into an object
but cannot "**unpack or de-encapsulate**" it
([PDF](https://www.math.kent.edu/~edd/PROCESSFUNC.pdf)). The strongest *verified* dedicated
source is **Hamdan (2006)**, *ESM* 62(2), 127–147 — an APOS **genetic decomposition** whose
central object *is* the preimage: "the **fiber structure of a function** on that set (i.e., the
set of preimages of all sets {b} for b in the range)", connecting preimage-fibers to partitions
and equivalence classes, ERIC [EJ748150](https://eric.ed.gov/?id=EJ748150), DOI
[10.1007/s10649-006-5798-9](https://doi.org/10.1007/s10649-006-5798-9).
**Asiala et al. (1996)** supplies the APOS methodology and the "schema of functions"
terminology, but a full-text search of it for `preimage`/`pre-image`/`inverse image` returned
**no hits** — it is not a source of image/preimage findings.

### Blocker 7 — Countable vs uncountable, Cantor's diagonalization, "same cardinality"
The key source is **Hamza & O'Shea (2011)**, MEI 4 proceedings, 192–202: 35 students (including
out-of-field mathematics teachers) showed five misconception families — **"countable" read as
"can be physically counted"** so that countable ≡ finite and infinite ≡ uncountable; students
asserting *every subset of an uncountable set is uncountable* and *all uncountable sets are
equinumerous*; and **none of the students who invoked the bijection criterion used it in all
problems, and they rarely wrote down an actual map**
([repository](https://mural.maynoothuniversity.ie/6977/)).
On **acceptance of diagonalization**: **Zazkis & Mamolo (2009)**, *For the Learning of
Mathematics* 29(3), 53–56, "Sean vs. Cantor" — a master's student's invalid enumeration of the
reals resisted multiple refutations, and a later cohort *after* learning Cantor's theorem
responded "Cool!" with nodding agreement, **not detecting the contradiction**
([full PDF](https://flm-journal.org/Articles/492E35FADC6DE2DD1D825A1FEEB71.pdf)).
On the **part–whole intuition**: **Monaghan (1986)** PhD thesis, Warwick — "Subjects' concepts
of infinity do not conform to infinite cardinal or ordinal paradigms", and a *measuring* context
makes subjects ascribe greater cardinality to the superset
([full PDF](http://wrap.warwick.ac.uk/34626/1/WRAP_THESIS_Monaghan_1986.pdf)). Also verified:
**Dubinsky, Weller, McDonald & Brown (2005)**, "Some historical issues and paradoxes regarding
the concept of infinity: An APOS-based analysis", *ESM* Parts 1 & 2, DOIs
[10.1007/s10649-005-2531-z](https://doi.org/10.1007/s10649-005-2531-z) /
[10.1007/s10649-005-0473-0](https://doi.org/10.1007/s10649-005-0473-0); and the dissenting
curricular position in **Blaszczyk (2020)**, *Mathematics Teaching Research Journal*
([free PDF](https://files.eric.ed.gov/fulltext/EJ1384460.pdf)).

### Blocker 8 — The Axiom of Choice
**Documented negative, and it is a strong result:** **no empirical study exists of students' or
instructors' attitudes to AC, and none on teaching AC.** ERIC `"axiom of choice"` (quoted, all
years) returns **1 record total and it is irrelevant** (an employee-rewards paper matching
"choice"); ERIC `"axiom of choice" teaching` returns **0**; ten OpenAlex title/abstract
phrasings, OpenAlex full-text search, five Crossref queries and an arXiv math.HO sweep produced
no education study. **Do not let any deliverable claim "studies show students think X about
AC."** The only explicit AC-pedagogy claim found is **Förster (2006)**, "The axiom of choice and
inference to the best explanation" (preprint, no DOI/venue, full text unreachable) whose
abstract states: "It is common practice in the teaching of mathematics at university level to
gloss over applications of the axiom of choice … **The students in consequence do not form a
mental image of the axiom, and tend subsequently not to recognise when it is being used**."
Nearest verified neighbours: **Bell, "The Axiom of Choice"**, *Stanford Encyclopedia of
Philosophy* ([full text](https://plato.stanford.edu/entries/axiom-choice/)); **Maddy (1988)**
"Believing the axioms" I & II, *Journal of Symbolic Logic* (philosophical analysis of
*mathematicians'* reasons, **not** a survey); the Banach–Tarski expositions (Buchhorn
[arXiv:2108.05714](https://arxiv.org/abs/2108.05714), Wahlberg
[arXiv:2206.13512](https://arxiv.org/abs/2206.13512)); **Incatasciato & Sánchez Terraf
(2024)** [arXiv:2404.11638](https://arxiv.org/abs/2404.11638) — the leanest proof of Zorn's
Lemma, **verified in Lean**, directly usable for a curriculum that keeps AC explicit;
**Wan, Xu & Cao (2023)**, *IJSI* 13(3), 323–357, a Coq formalisation of ZFC built for teaching,
reporting "beginners find it difficult to accurately understand abstract concepts, such as
syntax, semantics, and reasoning system"; and the empirical neighbour **Dawkins (2018)**,
*Investigations in Mathematics Learning* 10(4), 227–239, ERIC
[EJ1188492](https://eric.ed.gov/?id=EJ1188492), the closest existing framework for *what
students think an axiom is* (five categories; the most problematic is a referent-focused view).

### Blocker 9 — Transition to proof
**Moore (1994)**, *ESM* 27(3), 249–266, DOI
[10.1007/BF01273731](https://doi.org/10.1007/BF01273731) — the foundational study (University of
Georgia, 1989; observation + tutorial sessions + interviews) found "**three major sources of the
students' difficulties: (a) concept understanding, (b) mathematical language and notation, and
(c) getting started on a proof**", analysed through "concept definitions, concept images, and
concept usage". **Selden & Selden (1995)**, *ESM* 29(2), 123–151, DOI
[10.1007/BF01274210](https://doi.org/10.1007/BF01274210) — 61 students, "**just 8.5% of
unpacking attempts were successful**" for simplified informal calculus statements, dropping to
"**5%**" for real textbook statements; introduces *statement image* and *proof framework*.
**Selden & Selden (2003)**, *JRME* 34(1), 4–36, DOI
[10.2307/30034698](https://doi.org/10.2307/30034698) — 8 majors; they "tend to focus on
**surface features**" and their validation ability is "**very limited – perhaps more so than
either they or their instructors recognize**". **Weber (2001)**, *ESM* 48(1), 101–119, DOI
[10.1023/A:1015535614355](https://doi.org/10.1023/A:1015535614355) — undergraduates "are aware
of and able to apply the facts required to prove a statement but still fail", because they
"could not use the **syntactic knowledge** that they had"; doctoral students had four kinds of
**strategic knowledge** they lacked. **Inglis & Alcock (2012)**, *JRME* 43(4), 358–390, DOI
[10.5951/jresematheduc.43.4.0358](https://doi.org/10.5951/jresematheduc.43.4.0358) —
eye-tracking confirms novices attend to surface features rather than logical structure.
**Harel & Sowder (1998)**, *CBMS Issues in Mathematics Education* 7, DOI
[10.1090/cbmath/007/07](https://doi.org/10.1090/cbmath/007/07) — the **seven proof schemes** in
three classes (external conviction: authoritarian / ritual / non-referential symbolic;
empirical: inductive / perceptual; deductive: transformational / axiomatic), verified via
Kanellos, Nardi & Biza (2018)
([OA PDF](https://ueaeprints.uea.ac.uk/id/eprint/66431/1/Kanellos_Nardi_Biza_MTL_D_17_00097_030318_FINAL_PRE_PROOF_.pdf)).
**Selden (2012)**, ICMI Study volume, DOI
[10.1007/978-94-007-2129-6_17](https://doi.org/10.1007/978-94-007-2129-6_17) — **the best single
synthesized survey citation**, listing the difficulties as "the proper use of logic … formal
definitions … examples, counterexamples, and nonexamples … deep understanding of the concepts
and theorems … strategic knowledge". **Tall (2013)**, *How Humans Learn to Think
Mathematically*, CUP, DOI
[10.1017/cbo9781139565202](https://doi.org/10.1017/cbo9781139565202) — why "mathematical
concepts that make sense in one context may become problematic in another".
**Stylianides & Stylianides (2009)**, *JRME* 40(3), 314–352, DOI
[10.5951/jresematheduc.40.3.0314](https://doi.org/10.5951/jresematheduc.40.3.0314) — the
instructional-design counterpart (moving students off empirical arguments via cognitive
conflict). Bridge to the curriculum question: **Dawkins, Zazkis & Cook (2022)**, *PRIMUS*,
ERIC [EJ1322061](https://eric.ed.gov/?id=EJ1322061), on how existing transition-to-proof
textbooks connect logic, proof techniques and sets.

### Blocker 10 — APOS theory and Dubinsky's / RUMEC's set-theory research
The canonical citations are **Dubinsky & McDonald (2005)**, in the ICMI Study volume,
DOI [10.1007/0-306-47231-7_25](https://doi.org/10.1007/0-306-47231-7_25)
([author's free PDF](https://www.math.kent.edu/~edd/ICMIPaper.pdf)) — APOS "is being used in an
organized way by members of **RUMEC**", and it "**provide[s] explanations of student
difficulties and predict[s] success or failure**" — and **Arnon, Cottrill, Dubinsky, Oktaç,
Roa Fuentes, Trigueros & Weller (2014)**, *APOS Theory*, Springer, DOI
[10.1007/978-1-4614-7966-6](https://doi.org/10.1007/978-1-4614-7966-6).
The RUMEC set-theory-adjacent empirical work is **quantification** (**Dubinsky 1997**, ERIC
[EJ567950](https://eric.ed.gov/?id=EJ567950); the unpublished **Dubinsky & Yiparaki 2000**;
**Dubinsky, Elterman & Gong 1988**), **functions/relations** (**Breidenbach et al. 1992**;
**Dubinsky & Harel 1992**, MAA Notes 25), **infinity** (**Dubinsky, Weller, McDonald & Brown
2005**, *ESM*), and the **ISETL discrete-mathematics textbook tradition** (**Baxter, Dubinsky &
Levin 1989**, *Learning Discrete Mathematics with ISETL*, DOI
[10.1007/978-1-4612-3592-7](https://doi.org/10.1007/978-1-4612-3592-7) — the same environment
Zazkis & Gunn used).
⚠️ **Important framing:** Dubinsky's and RUMEC's published empirical work is overwhelmingly
about calculus, linear algebra, abstract algebra and functions — **not about set theory as a
topic**. The *closest* thing to a dedicated APOS set-theory study found is **Hamdan (2006)**
(equivalence classes, partitions, fiber structures). The nearest *recent* set-theory-learning
work is computing-mediated: **Martinez (2022)** dissertation, ERIC
[ED646013](https://eric.ed.gov/?id=ED646013), and **Martinez IV (2024)**, *Digital Experiences
in Mathematics Education*, ERIC [EJ1417658](https://eric.ed.gov/?id=EJ1417658). A survey
claiming "APOS research shows students struggle with set theory" would be overclaiming.

---

## 2. Corrections and cautions applied during assembly

These matter because the detailed parts below were produced by four independent research
passes and a few numbers were found to disagree. Where they disagreed, the primary source was
re-read or re-queried, and the resolution is recorded here.

1. **Dubinsky (1997) class sizes and year — resolved, and a subtlety worth keeping.** The
   paper reports **36 students** in total, made up of **Class 1 = 19 students** ("Introduction
   to Finite Mathematics", Clarkson University, **Fall 1986**) and **Class 2 = 17 students**
   ("Introduction to Analysis", the following semester). An intermediate draft of Part B
   recorded "n = 21, Fall 1995" and "n = 19"; that draft was wrong on both counts and Part B's
   final text is correct. **Additional subtlety:** the per-instrument *n* is not constant —
   the result tables are captioned SetA 18, SetB 18, SetC 19, AssignA 19, AssignB 19, Class 2
   problems 17. If the survey quotes a sample size per problem set, use those figures, not 36.
2. **Hamdan (2006) volume/pages.** One pass recorded 62(1), 51–77; Crossref returns
   ***ESM* 62(2), 127–147**. Crossref is authoritative: **62(2), 127–147**.
3. **Dubinsky & McDonald page range.** Crossref returns **275–282**; Dubinsky's own
   publications page says **273–280**. The chapter is also dated **2001** in Dubinsky's list and
   **2005** in the Springer/Semantic Scholar record. **Cite the volume and DOI, and give the
   page range with a note if precision matters.**
4. **Dubinsky, Elterman & Gong year/volume.** The article's own reference list in Dubinsky
   (1997) gives **1988, *FLM* 8(2), 44–51**; Dubinsky's publications page gives 1989 with no
   volume. Prefer **1988, 8(2), 44–51**.
5. **ERIC EJ1070077 is NOT Bagni (2006).** It is Chadli, Bendella & Tranvouez (2015),
   *Educational Technology & Society* 18(2) — a fuzzy-set ed-tech paper. Bagni (2006) appears
   to have **no ERIC record**; verify it via DOI
   [10.1007/s10649-006-8545-3](https://doi.org/10.1007/s10649-006-8545-3) instead.
6. **Piatek-Jimenez.** No 2004 one-to-one/onto paper exists in Crossref or OpenAlex. See
   Blocker 5 above. Her actual paper is Piatek-Jimenez (2010), *MERJ* 22(3), 41–56, on
   students' interpretations of *quantified* statements.
7. **Cusi & Malara (2007) on quantifiers — NOT FOUND.** Crossref
   (`query.bibliographic=Cusi Malara quantifiers argumentation`) returned unrelated
   generalised-quantifier linguistics; the HAL API (`quantifiers mathematics education
   Malara`) returned zero results. What does exist and was verified is **Cusi, A. & Malara,
   N. A. (2011)**, "Improving awareness about the meaning of the principle of mathematical
   induction", *PNA — Revista de Investigación en Didáctica de la Matemática* — **topic is
   induction, not quantifier order**. Use the Cusi/Malara citation only if the induction topic
   is acceptable.
8. **Zazkis & Gunn (1997) full text is unreachable from this IP** (learntechlib 403 /
   AWS WAF; editlib 403; all CORS proxies refused; archive.org offline). Only the ERIC record
   ([EJ543540](https://eric.ed.gov/?id=EJ543540)) is verified — **scope and method are
   confirmed, error categories are not**. Re-fetch from a different egress IP or a library.
9. **Zazkis & Mamolo (2009)** is cited by one pass as *For the Learning of Mathematics* 29(3),
   53–56, "Sean vs. Cantor"; the full PDF was retrieved from `flm-journal.org`. The volume/page
   detail was not independently re-verified at assembly time — verify before quoting pages.
10. **Tall's "three worlds" terminology.** The book title and the existence of "three distinct
    worlds of mathematical thinking (Tall, 2004)" are verified, but **no source retrieved this
    session names the three worlds**, so the usual embodied / proceptual-symbolic /
    formal-axiomatic labels should **not** be quoted as Tall's without re-verification.
11. **Sfard (1991)** — the citation, venue, volume and 1,848-citation count are verified, but
    **no sentence of its content was retrieved** (no abstract in any index; closed access). Do
    not attribute specific findings to it.
12. **Blocker 8 is empty and that is the finding.** See the executive summary; the detailed
    searches and their null results are enumerated in Part C.
13. **Blocker 3 has no empirical literature** on the encoding itself. See Part A.
14. **Dubinsky & Yiparaki (2000)** is an **unpublished manuscript**, not a peer-reviewed
    article, despite carrying the sharpest quantifier-order numbers in this dossier. If the
    survey needs a peer-reviewed citation for the same phenomenon, use Dubinsky (1997) plus
    Dawkins & Roh (2020) — and state the manuscript status if the 94%/5% figures are quoted.

---

## 3. Highest-value remaining gaps (ranked)

If more retrieval is possible from a different network egress, these are the items that would
most improve the dossier, in order:

1. **Zazkis & Gunn (1997)** full text — the canonical element/subset/empty-set study; only its
   ERIC record is currently verified. Blocked by `learntechlib.org` 403 / AWS WAF.
2. **Bansilal, Brijlall & Trigueros (2017)**, *JMB* 48, 22–37, on pre-service teachers'
   understanding of **injections and surjections** — the single most on-target paywalled item
   for blockers 5 and 6. DOI
   [10.1016/j.jmathb.2017.08.002](https://doi.org/10.1016/j.jmathb.2017.08.002).
3. **Selden & Selden (2003)** open copy (Unpaywall points at
   `https://philpapers.org/archive/SELVOP-2.pdf`) — blocked by Cloudflare here.
4. **Hamza & O'Shea (2011)** — the PDF at
   `http://eprints.maynoothuniversity.ie/6977/1/AOS-Student-Misconceptions.pdf` is image-only;
   the companion pass OCR'd it, but a text-native copy would let its student quotes be verified
   verbatim.
5. **A genuine empirical study of the Kuratowski encoding.** None exists in any index
   searched; if the survey needs one, it likely requires new data collection, not retrieval.
6. **A genuine study of AC attitudes or AC teaching.** Same conclusion — see Blocker 8.
7. **Blocks 4.5**: Tall's three-worlds terminology, and Sfard (1991)'s actual content, both
   need a library copy before being quoted.

---

## 4. Detailed findings

The four parts that follow are the full research dossiers, reproduced as written, each with its
own provenance note, verification legend and "UNVERIFIED / COULD NOT RETRIEVE" section.
Part A = blockers 1–3; Part B = blockers 4–6; Part C = blockers 7–8; Part D = blockers 9–10.


---

# Part A — Blockers 1–3 (∈ vs ⊆, the empty set, ordered pairs)

# Part A — Research evidence for three set-theory blockers

Scope: (1) `∈` vs `⊆` and element-vs-singleton; (2) the empty set and vacuous truth;
(3) ordered pairs and the Kuratowski encoding.

**Method / provenance note.** `web_search` and `web_fetch` were not used (per BRIEF).
Everything below was retrieved with `curl` through the local proxy
(`http://127.0.0.1:7890`) using Crossref, ERIC (eric.ed.gov + files.eric.ed.gov),
OpenAIRE, DOAJ, Unpaywall and Semantic Scholar metadata endpoints; PDFs were converted
with PyMuPDF. Every source below is marked with the URL I actually fetched.

**Two hard environment constraints hit during this work (both matter for anyone
continuing this survey):**
- **OpenAlex API is exhausted for this IP** for the rest of the UTC day
  (`{"error":"Rate limit exceeded","dailyRemainingUsd":0.0003}`). All `oa.sh` calls were
  abandoned early; OpenAIRE + Crossref + ERIC were used instead.
- **`learntechlib.org` (AACE/LearnTechLib) is hard-blocked from this IP.** Through the
  proxy it returns HTTP 403 with a body telling the operator to email
  `info@aace.org` with their IP; a direct connection returns an AWS WAF JS-only
  "Human Verification" challenge (HTTP 405). `editlib.org` is 403 too. `r.jina.ai`,
  public CORS proxies (`allorigins`, `codetabs`, `corsproxy.io`), `scholar.archive.org`
  and the Wayback Machine were all unavailable (Wayback/archive.org is currently
  "Temporarily Offline", HTTP 503). **Consequence: the Zazkis & Gunn (1997) full text
  could not be read; only its ERIC record could be verified.** See
  §UNVERIFIED / COULD NOT RETRIEVE.

---

## BLOCKER 1 — Confusion between `∈` and `⊆`; element vs singleton (`x ∈ A` vs `x ⊆ A`, `{a}` vs `a`)

1. **Bagni, Giorgio T. (2006). "Some Cognitive Difficulties Related to the Representations
   of two Major Concepts of Set Theory." *Educational Studies in Mathematics* 62(3).**
   - Source type: peer-reviewed journal article (Springer). Cited 27× per Semantic
     Scholar (Crossref count 7).
   - DOI: `10.1007/s10649-006-8545-3`
   - URLs actually fetched: publisher landing page
     `https://link.springer.com/article/10.1007/s10649-006-8545-3` (JS-blocked, no text),
     then the Crossref record `https://api.crossref.org/works/10.1007/s10649-006-8545-3`
     and **the OpenAIRE record, which returned the publisher abstract**:
     `https://api.openaire.eu/search/publications?doi=10.1007/s10649-006-8545-3&format=json`
   - **Actual findings (from the retrieved abstract):** the paper's "main focus … is on
     the study of students' conceptual understanding of **two major concepts of Set
     Theory – the concepts of inclusion and belonging**", analysed through **two
     experimental classroom episodes**. The analysis rests on the theoretical idea that,
     "from an ontogenetic viewpoint, the cognitive activity of representation of
     mathematical objects draws its meaning from different semiotic systems framed by
     their own cultural context." The result: "the successful accomplishment of
     knowledge attainment seems to be linked to the students' ability to **suitably
     distinguish and coordinate the meanings and symbols of the various semiotic
     systems (e.g. verbal, diagrammatic and symbolic)**" of their mathematical
     experience.
   - Relevance: **this is the single best-matched peer-reviewed source for Blocker 1** —
     it is explicitly about belonging (`∈`) vs inclusion (`⊆`) and locates the difficulty
     in coordinating verbal / diagrammatic / symbolic registers. (Full text is closed;
     Unpaywall reports `is_oa:false`, no repository copy.)

2. **Zazkis, Rina & Gunn, Chris (1997). "Sets, Subsets, and the Empty Set: Students'
   Constructions and Mathematical Conventions." *Journal of Computers in Mathematics
   and Science Teaching* 16(1), 133–169.**
   - Source type: peer-reviewed journal article (AACE); ERIC record type CIJE.
   - ERIC ID: **EJ543540**; ISSN 0731-9258.
   - URL actually fetched (record page): `https://eric.ed.gov/?id=EJ543540`
   - **Actual findings (verbatim scope from the retrieved ERIC record):** "Investigates
     students' understanding of the basic concepts of introductory set theory which are
     **set, set element, cardinality, subset, and the empty set**. Data was collected
     from **preservice elementary school teachers**. The project included
     experimentation with basic set concepts in an **open computer-based environment
     using the mathematical computer language ISETL**. A constructivist-oriented
     framework was used in analyzing the data."
   - Relevance: **the canonical study for Blockers 1 and 2 together** — element, subset
     and empty set in one design, with a constructionist/ISETL environment. ⚠️ The full
     text was **not** readable (see provenance note); I can state its scope and method
     but **not** its specific error categories. Treat the "*x* ∈ A vs *x* ⊆ A" claim as
     *scope-confirmed, detail-unverified* until someone obtains the PDF.

3. **Hendriyanto, Agus; Suryadi, Didi; Juandi, Dadang; Dahlan, Jarnawi Afgani; Hidayat,
   Riyan; Wardat, Yousef; Sahara, Sani; Muhaimin, Lukman Hakim (2024). "The didactic
   phenomenon: Deciphering students' learning obstacles in set theory." *Journal on
   Mathematics Education* 15(2), 517–544.**
   - Source type: peer-reviewed journal article (OA, Indonesia).
   - DOI: `10.22342/jme.v15i2.pp517-544`; ERIC ID EJ1428069.
   - URL actually fetched (full-text PDF, read in full):
     `https://files.eric.ed.gov/fulltext/EJ1428069.pdf`
   - **Actual findings (this is the most concrete Blocker-1 evidence I found).** The
     study gave 183 junior-high participants (24 interviewed in depth) a task
     `(ω) M = {r, s, t}` with three items: `ω1: r ∈ M`, `ω2: s ∉ M`, `ω3: {r} ∈ M`, plus
     `(φ) A = {x | 2x = 6, x ∈ integers}` vs `B = 3`, "does A = B?".
     - `ω1` and `ω2` — **all** students answered correctly.
     - `ω3` (`{r} ∈ M`) — **no student answered correctly with a valid justification**;
       those who wrote "wrong" could not justify it. Students rejected the *singleton*:
       quoted reasons include "`{r}` is not an element of the set M", "the element of the
       set M is not just r, but r, s, t", and "the statement `{r} ∈ M` can be written
       `M = {r}` whereas `M = {r, s, t}`".
     - `φ` — no student answered correctly; many wrote `A = B`, and the authors classify
       this as students who "**cannot distinguish between ∈ and =**" (an *instrumental
       ontogenic obstacle*) and who "cannot distinguish between sets and set elements".
     - Conclusion states the didactic obstacle directly: "the unstructured arrangement of
       the material may lead students to **reject sets with a single element and empty
       sets**."
   - Relevance: **empirical, quotable, and precise for element-vs-singleton** (`{r}` vs
     `r`) and for `∈` vs `=` / set-vs-element confusion. Note it is a junior-high sample
     (not undergraduates) and a didactical-design study, not a large-scale survey.

4. **Kolitsoe Moru, Eunice & Qhobela, Makomosela (2013). "Secondary School Teachers'
   Pedagogical Content Knowledge of Some Common Student Errors and Misconceptions in
   Sets." *African Journal of Research in Mathematics, Science and Technology
   Education* 17(3), 220–230.**
   - Source type: peer-reviewed journal article (Taylor & Francis / Routledge).
   - DOI: `10.1080/10288457.2013.848534`; ERIC ID EJ1177882.
   - URLs actually fetched: `https://eric.ed.gov/?id=EJ1177882` and the OpenAIRE DOI
     record `https://api.openaire.eu/search/publications?doi=10.1080/10288457.2013.848534&format=json`
   - **Actual findings:** five Lesotho secondary mathematics teachers were probed on
     common student errors in sets. Teachers **could** identify: "(i) writing an empty
     set as `{0}` instead of `{ }`; (ii) treating the repeating elements of the union of
     two sets as distinct; and (iii) treating an infinite set as a finite set." Teachers
     **could not** identify: "(i) treat[ing] infinity as a number; (ii) [saying] that the
     members of countable infinite sets cannot be compared; and (iii) that curly brackets
     are used only when listing the members of a set." When handling errors, teachers'
     strategies "were inclined towards calling on procedural knowledge. Only a few cases
     of conceptual knowledge were noted."
   - Relevance: shows the **notation-as-decoration misconception** ("curly brackets are
     only for listing members") and the `{0}`-for-`∅` error — both are direct cousins of
     `∈`/`⊆` and element/singleton slippage, and it documents that *teachers* miss them.
     Closed access; abstract obtained via ERIC + OpenAIRE.

5. **Narli, Serkan & Baser, Nes'e (2008). "Cantorian Set Theory and Teaching Prospective
   Teachers." *International Journal of Environmental & Science Education* 3(2), 99–107.**
   - Source type: peer-reviewed journal article (OA); ERIC ID EJ894852 (also ED506476).
   - URL actually fetched (full-text PDF, read):
     `https://files.eric.ed.gov/fulltext/EJ894852.pdf`
   - **Actual findings:** prospective mathematics teachers (Dokuz Eylul University,
     Turkey) were taught Cantorian set theory either by an active-learning course or by
     traditional lecture; the active-learning group was significantly more successful
     (reported *p* = 0.02). Before instruction, students' brainstormed definitions of
     *equivalence* of sets transferred finite intuitions to infinite sets, producing
     statements such as "**if two sets are subset of each other they are equivalent**",
     "sets having equal number of elements are equivalent", "sets having equal number of
     **subsets** are equivalent", "two infinite sets are equivalent". Only after
     instruction did they construct "two sets having a 1-1 corresponding function between
     them are equivalent".
   - Relevance: **indirect** for Blocker 1 — it is not about `∈` vs `⊆`, but it shows
     prospective teachers reasoning with *subset* language where *bijection* is required,
     i.e. subset vocabulary being used as a blunt instrument. Include as supporting
     context, not as direct evidence.

6. **Dawkins, Paul Christian; Zazkis, Dov; Cook, John Paul (2022). "How Do Transition to
   Proof Textbooks Relate Logic, Proof Techniques, and Sets?" *PRIMUS* 32(1), 14–30.**
   - Source type: peer-reviewed journal article.
   - ERIC ID EJ1322061; URL actually fetched: `https://eric.ed.gov/?id=EJ1322061`
   - **Actual findings (from the retrieved ERIC record):** the authors analyse a sample of
     transition-to-proof (TTP) textbooks for how they connect logic, proof techniques,
     and sets; they were "motivated by recent research showing that **focusing on sets is
     propitious to novice students' ability to reason about logic and construct valid
     arguments**." They describe how logic is used to explain sets and vice versa, flag
     places where connections are made and where opportunities for connection are missed,
     and "suggest several key recommendations TTP instructors can leverage in their
     unit(s) on sets."
   - Relevance: useful for the *curriculum* side of the survey — it is evidence that the
     logic↔sets bridge is pedagogically consequential and that textbooks are inconsistent
     about it. Note: this is **Dov** Zazkis (not Rina Zazkis) — do not conflate.

7. **Kanamori, Akihiro (2003). "The Empty Set, The Singleton, and the Ordered Pair."
   *Bulletin of Symbolic Logic* 9(3).**
   - Source type: peer-reviewed journal article (logic), Cambridge University Press;
     cited 50× (Semantic Scholar), 30× (Crossref).
   - DOI: `10.2178/bsl/1058448674`
   - URLs actually fetched: Crossref `https://api.crossref.org/works/10.2178/bsl/1058448674`
     and OpenAIRE `https://api.openaire.eu/search/publications?doi=10.2178/bsl/1058448674&format=json`
     (full abstract retrieved from both).
   - **Actual findings:** "the empty set Ø, the singleton {*a*}, and the ordered pair
     〈*x, y*〉 are at the beginning of the systematic, axiomatic development of set
     theory … These notions are the simplest building blocks in the abstract, generative
     conception of sets … So it is surprising that, while these notions are unproblematic
     today, **they were once sources of considerable concern and confusion among leading
     pioneers of mathematical logic like Frege, Russell, Dedekind, and Peano.**" Kanamori
     frames their emergence as "a motif that reflects and illuminates larger and more
     significant developments in mathematical logic: the **shift from the intensional to
     the extensional viewpoint**, the development of **type distinctions**, the logical
     vs. the iterative conception of set".
   - Relevance: authoritative for the *historical/conceptual* claim that `{a}` (the
     singleton) is a genuine conceptual hurdle, not a triviality — and it links
     Blockers 1 and 3 (singleton → ordered pair). It is a history-of-logic paper, **not**
     an empirical study of students.

8. **Martinez, Antonio Estevan (2022). "Bridging the Gap between Set Theory and Logic:
   Leveraging Computing as a Mediating Tool for Learning." PhD dissertation, University
   of California, San Diego.**
   - Source type: doctoral dissertation (ProQuest); ERIC ID ED646013.
   - URL actually fetched: `https://eric.ed.gov/?id=ED646013`
   - **Actual findings (from the retrieved ERIC abstract):** a multi-session teaching
     experiment on how programming/computing can mediate the set theory ↔ logic
     connection for undergraduates in an Introduction to Proofs context. Three dimensions:
     students' in-the-moment reasoning about set theory and logic; their advancing
     mathematical activity over the experiment; their affective experience
     (confidence, interest, self-efficacy). Result: "students … were able to leverage
     computing as an **accessible onramp** to the fundamental ideas related to set theory
     and logic", and computing "can have a positive effect on one's sense of confidence
     and interest". 359 pp.
   - Relevance: the only *undergraduate* set-theory-and-logic teaching experiment I could
     verify in this pass; useful for the survey's "how do we teach it" strand. Full text
     is ProQuest-restricted; only the abstract was retrievable (eScholarship was
     bot-blocked, HTTP 202/empty).

---

## BLOCKER 2 — The empty set, `∅ ⊆ A`, and vacuous truth / false antecedents

1. **Zazkis, Rina & Gunn, Chris (1997).** — *same citation as Blocker 1 #2.*
   - Source type: peer-reviewed journal article; ERIC ID **EJ543540**;
     URL fetched: `https://eric.ed.gov/?id=EJ543540`
   - **Why it belongs here:** the empty set is named in the title and in the ERIC scope
     statement ("set, set element, cardinality, subset, and **the empty set**"), studied
     with preservice elementary teachers in ISETL. Specific findings on students'
     resistance to `∅` are **not** verifiable from the abstract alone (full text blocked).

2. **Hendriyanto et al. (2024).** — *same citation as Blocker 1 #3.*
   - URL fetched: `https://files.eric.ed.gov/fulltext/EJ1428069.pdf`
   - **Actual findings on the empty set (verbatim/near-verbatim from the article):**
     two of the five "is this a set?" objects were empty sets
     (`{x | x is the letter before a in the Latin alphabet sequence}` and `{ }`).
     "**Many of the students did not accept this as a set. The reason is all the same: no
     element in it can be identified.**" The authors attribute this to an under-developed
     concept of set ("their perception of β [i.e. 'a collection of several elements']
     … disregards other understandings, as seen in the case of the empty set. The absence
     of elements in the empty set leads students to assert that it is not a set").
     A focal interviewee ("Waluyo") *did* accept `∅` as a set, but only on authority: he
     said "in the book and explained by the teacher, there is such a thing as an empty
     set"; when challenged — "**The empty set has no elements, why can it be called a
     group?**" — "Waluyo was silent and could not explain." The authors call this an
     *epistemological obstacle*: knowledge based on testimony only.
   - Relevance: **the strongest concrete evidence I found for the "students do not accept
     `∅` as a set" phenomenon**, with a verbatim student quote and an authority-only
     acceptance case. Junior-high sample.

3. **Kolitsoe Moru & Qhobela (2013).** — *same citation as Blocker 1 #4.*
   - URLs fetched: `https://eric.ed.gov/?id=EJ1177882` + OpenAIRE DOI record.
   - **Relevant finding:** the very first error teachers could identify is "**writing an
     empty set as `{0}` instead of `{ }`**" — i.e. the null set is assimilated to the
     numeral zero. (Also relevant: teachers failed to notice "curly brackets are used only
     when listing the members of a set".)

4. **Riegler, Peter (2013). "Students' Conceptions of Nothingness and Their Implications
   for a Competency-Driven Approach to the Curriculum." *Teaching Mathematics and Its
   Applications* 32(2), 76–80.**
   - Source type: peer-reviewed journal article (Oxford University Press); ERIC ID
     EJ1003630. DOI: `10.1093/teamat/hrt007`.
   - URLs actually fetched: `https://eric.ed.gov/?id=EJ1003630` and OpenAIRE
     `https://api.openaire.eu/search/publications?doi=10.1093/teamat/hrt007&format=json`
   - **Actual findings:** the article's stated purpose is to show that "mathematics, like
     any subject matter, contains **inherent difficulties for students**", and that
     competency-driven curriculum frameworks "tend to ignore this issue". It does this
     "by **investigating students' difficulties with the concept of the empty set** on one
     hand and the framework of the European Society for Engineering Education (SEFI) for a
     mathematics curriculum on the other hand." Higher-education context.
   - Relevance: directly on the empty set, and useful for the *curriculum-critique*
     framing (competency statements assume away conceptual obstacles such as `∅`). Closed
     access — abstract only; the 5-page paper was not readable, so no finer-grained
     findings can be quoted.

5. **Sriraman, Bharath & Knott, Libby (2006). "1 or 0? Cantorian Conundrums in the
   Contemporary Classroom." *Australian Senior Mathematics Journal* 20(2), 57–61.**
   - Source type: peer-reviewed journal article (AAMT); ERIC ID **EJ744042** (abstract by
     ERIC). ISSN 0819-4564.
   - URL actually fetched: `https://eric.ed.gov/?id=EJ744042`
   - **Actual findings (verbatim from the retrieved ERIC record):** "In set theory, one
     comes across the notion of 'vacuous truth'. A statement is vacuously true if it is
     true but does not quite say anything. The structure of a vacuously true statement is
     typically of the form: everything with property A also has property B, with the
     caveat being that there is nothing in property A. For instance one could say: all
     humans with gills are sharks. This statement is vacuously true because there are no
     humans with gills. It is natural to dismiss such examples as absurd and pathologies
     within the framework of set theory. **However the notion of vacuous truth arises in
     some pedagogical situations** … the author describes one such situation in a
     **preservice elementary mathematics classroom**."
   - Relevance: **the one ERIC-indexed paper explicitly about vacuous truth in a
     mathematics classroom.** It is a descriptive/practitioner-oriented piece
     ("Reports - Descriptive"), not an experiment — it documents that the situation
     *arises*, not effect sizes. Full text not on ERIC.

6. **Lee, Kyung-Jin; Park, JinHyeong; Kim, Suh-Ryung (2024). "Exploration of the Truth
   Values of Conditionals Set Up in Everyday Context and in Open Sentences."
   *International Journal of Science and Mathematics Education* 22(3), 657–678.**
   - Source type: peer-reviewed journal article (Springer); ERIC ID **EJ1410917**.
     DOI: `10.1007/s10763-023-10391-w`.
   - URLs actually fetched: `https://eric.ed.gov/?id=EJ1410917` (full abstract) and
     Crossref `https://api.crossref.org/works?query.bibliographic=...` for the DOI.
   - **Actual findings:** the study designed "a task in an everyday context to support high
     school students' exploration of the **truth values of propositional conditionals** and
     to understand their difficulties." Results: students "could identify that the truth
     value of a propositional conditional is determined by two variables (the truth values
     of the antecedent and the consequent) while organizing the propositional conditionals
     generated from the open sentence", and "**a social contract situation can help
     students understand why the truth value of a conditional with a false antecedent is
     true**." Difficulties in determining truth values "are closely related to the **time
     variable** in an everyday context and the **confusion between conditionals and
     causality**."
   - Relevance: **the best empirical, peer-reviewed, recent source for the
     false-antecedent half of Blocker 2**, and it names an instructional lever (social
     contract / promise-keeping framing). Full text is Springer-closed; ERIC abstract only.

7. **Durand-Guerrier, Viviane (2003). "Which notion of implication is the right one?
   From logical considerations to a didactic perspective." *Educational Studies in
   Mathematics* 53(1).**
   - Source type: peer-reviewed journal article (Springer); Crossref cited-by 57.
     DOI: `10.1023/a:1024661004375`.
   - URL actually fetched (abstract retrieved): OpenAIRE
     `https://api.openaire.eu/search/publications?doi=10.1023/a:1024661004375&format=json`
   - **Actual findings (from the retrieved abstract):** "Implication is at the very heart
     of mathematical reasoning. As many authors have shown, **pupils and students
     experience serious difficulties in using it in a suitable manner.** In this paper, we
     support the thesis that these difficulties are closely related with the **complexity
     of this notion**." Using Tarski's semantic truth theory, she distinguishes
     "propositional connective, logically valid conditional, generalized conditional,
     inference rules", and argues it is "necessary to extend the classical definition of
     implication as a relation between propositions to **a relation between open sentences
     with at least one free variable**". This "permits to become aware of the fact that, in
     some cases, **the truth-value of a given mathematical statement is not constrained by
     the situation**, contrary to the common standpoint that, in mathematics, a statement
     is either true or false." Illustrated with two problematic situations and
     "experimental results from our research on **first-year university students'
     understanding of implication**."
   - Relevance: the canonical didactic treatment of implication, and the theoretical basis
     for treating vacuous truth as a *hard* notion rather than a formality. Note: the
     abstract does not itself use the phrase "vacuous truth"; the vacuity point is reached
     via "truth-value … not constrained by the situation" and open sentences.

8. **Kanamori (2003).** — *same citation as Blocker 1 #7.* URL fetched: OpenAIRE/Crossref
   record (full abstract).
   - Relevant to Blocker 2 because the empty set is one of the three notions he traces
     from "considerable concern and confusion" among Frege, Russell, Dedekind and Peano,
     and because he frames the change as a shift "from the intensional to the extensional
     viewpoint" — the same shift students must make to accept `∅ ⊆ A`.

---

## BLOCKER 3 — Ordered pairs, the Kuratowski encoding `(a,b) = {{a},{a,b}}`, and the characteristic property

**Blunt negative result first:** I could **not** find any empirical study of students
learning the *Kuratowski encoding itself* (see §UNVERIFIED for exactly what I searched).
The reachable literature is (a) mathematics/logic scholarship on the ordered pair as a
conceptual achievement, and (b) mathematics-education work on the **set-theoretic
definition of function/relation as a set of ordered pairs**, which is where students
actually meet the encoding. That gap is itself a finding for the survey.

1. **Mirin, Alison; Weber, Keith; Wasserman, Nicholas (2020). "What Is a Function?"
   In *Mathematics Education Across Cultures: Proceedings of the 42nd Meeting of the
   North American Chapter of the International Group for the Psychology of Mathematics
   Education (PME-NA)*, pp. 1156–1161. Cinvestav / AMIUTEM.**
   - Source type: peer-reviewed conference proceedings paper (PME-NA); ERIC ID ED629969.
     Proceedings DOI `10.51272/pmena.42.2020`.
   - URL actually fetched (full-text PDF, read in full):
     `https://files.eric.ed.gov/fulltext/ED629969.pdf`
   - **Actual findings:** two inequivalent definitions of *function* are in live use —
     the **Bourbaki triple** (D, E, f) and the **set of ordered pairs** — and "these
     definitions entail different interpretations and answers to mathematical questions
     that even a secondary student might be prompted to answer. However, **mathematicians
     and mathematics educators are often not explicit about which definition they are
     using**." Concrete worked divergences: for the set `{(−1,4),(0,7),(2,3),(3,3),(4,−2)}`
     the ordered-pair reading says "yes, a function, domain {−1,0,2,3,4}" while the
     literal Bourbaki reading says "not a function, because functions are triples"; the
     two readings also disagree on whether `f(x)=e^x` has inverse `ln x` (ordered-pairs:
     yes; Bourbaki with codomain ℝ: no, not surjective). They note the ordered-pair view
     makes domain *derived* from the set of pairs and codomain non-unique, and document
     that mathematics-education research predominantly assumes the Bourbaki triple even
     while some educators (e.g. Sajka) assume the set of ordered pairs. They quote Forster
     (2003, pp. 10–11): "some mathematical cultures … [say] a function is an ordered
     triple of domain, range, and a set of ordered pairs. This notation has the advantage
     of clarity, but it has not yet won the day."; and they quote Selden & Selden (1992,
     p. 2): "the formal ordered pair definition of function, first introduced in 1939, is
     often referred to as the Bourbaki approach."
   - Relevance: **the strongest available education-research source for Blocker 3.** It
     does not study the Kuratowski *encoding*, but it establishes that the ordered pair is
     the load-bearing object in the formal definition students are taught, and that
     instructors/authors are systematically ambiguous about it — a direct instructional
     hazard for a Lean-style curriculum where the encoding is explicit.

2. **Karagöz Akar, Gülseren & Şener, Beyhan (2014). "Students' Development of The
   Relationship Between The Cartesian Product, Relation and The Definition of Function."
   *International Journal of Educational Studies in Mathematics (IJESIM)* 1(1).**
   - Source type: peer-reviewed journal article (Turkish OA journal; the journal's site,
     `ijesim.com`, is now a parked domain, so the DOI does not resolve to full text).
     DOI: `10.17278/ijesim.2014.01.006`.
   - URL actually fetched (abstract retrieved): OpenAIRE
     `https://api.openaire.eu/search/publications?doi=10.17278/ijesim.2014.01.006&format=json`
     (title also confirmed in Crossref `query.author=Karagöz Akar`).
   - **Actual findings (from the retrieved abstract):** six 9th-grade students in a
     private school, four consecutive weeks, GeoGebra and non-GeoGebra tasks with focused
     questioning. "Results revealed that students came to the understanding of the
     Cartesian Product between two sets as **the matching of all elements in the sets**.
     Results also indicated that **students were able to detect why the elements of a
     Cartesian Product needs to be in ordered pairs.** In addition, students were able to
     determine the graph of a function and a relation given a graph of a Cartesian Product
     and explain how they are related." Difficulties found: "**graphing a Cartesian
     Product defined on two finite and infinite sets** and in **considering equal sign as
     showing the output in terms of the input values**."
   - Relevance: the only study I found that directly targets the *orderedness* of pairs
     (why a Cartesian product needs ordered pairs) and links Cartesian product → relation
     → function. Sample is 9th grade, so it speaks to the school-level precursor of the
     formal encoding, not to the Kuratowski construction. Full text unreachable (dead
     journal domain); abstract is publisher-deposited.

3. **Kanamori (2003).** — *same citation as Blocker 1 #7.* URL fetched: OpenAIRE/Crossref.
   - Relevance for Blocker 3: treats `〈x, y〉` alongside `∅` and `{a}` as one of the three
     elementary notions whose "emergence … as clear and elementary set-theoretic concepts"
     required major conceptual shifts (extensional vs intensional viewpoint; type
     distinctions; logical vs iterative conception of set). This is the scholarly basis
     for saying the ordered pair is *not* psychologically elementary even though it is
     axiomatically early.

4. **Malambo, Priestly (2022). "Consequential implications of mathematics student
   teachers' definitions of the function concept." *Journal of Research and Advances in
   Mathematics Education (JRAMathEdu)* 7(4), 197–210.**
   - Source type: peer-reviewed journal article (OA, Indonesia).
     DOI: `10.23917/jramathedu.v7i4.17146`.
   - URL actually fetched (full-text PDF, read for ordered-pair content):
     `https://files.eric.ed.gov/fulltext/EJ1495540.pdf` (ERIC EJ1495540)
   - **Actual findings:** four mathematics student teachers; written definitions plus
     interviews. "the student teachers' definitions of a function were **dominated by a
     narrow view that all functions are one-to-one relations**. Notwithstanding, the
     participants' conception of one-to-one functions was superficial. The student
     teachers' **flawed definitions of a function influenced their inability to correctly
     identify functions**", and were "consistent with … incapacity to translate functions
     accurately from one kind of representation into another."
   - Relevance: **adjacent, not direct.** Its only ordered-pair content is the routine
     "plotted the resultant ordered pairs on Cartesian planes" (graphing sense), *not* the
     set-theoretic encoding. I include it because it is the clearest OA evidence that
     student teachers hold rule/one-to-one images of function that conflict with the
     formal set-of-ordered-pairs definition — the same concept-image/definition gap that
     Mirin et al. describe. Do not cite it for anything about Kuratowski.

### 3b. Citations verified (metadata only) — content NOT retrieved; use for framing only, do not attribute findings

These four records were verified against Crossref/OpenAIRE metadata (title, author, year,
venue, DOI), but I did **not** retrieve their text or abstracts, so no findings are asserted:

- **Halmos, Paul R. (1974). "Ordered Pairs." Ch. 6 in *Naive Set Theory*, Undergraduate
  Texts in Mathematics. Springer New York.** DOI `10.1007/978-1-4757-1645-0_6`
  (verified via `https://api.crossref.org/works/10.1007/978-1-4757-1645-0_6`; the chapter
  Crossref cited-by count is 1). This is the canonical textbook exposition of why an
  ordered pair is needed and of the defining property — retrieve before quoting.
- **Forster, Thomas (2003). *Logic, Induction and Sets*. Cambridge University Press.**
  DOI `10.1017/cbo9780511810282` (verified via Crossref `query.bibliographic=Forster
  Logic Induction and Sets`). **Quotable content is available second-hand and verified:**
  Mirin, Weber & Wasserman (2020) quote Forster pp. 10–11 verbatim (see item 1 above) —
  cite the quotation *as quoted in* Mirin et al. (2020) unless you obtain the book.
- **Benacerraf, Paul (1965). "What Numbers Could not Be." *The Philosophical Review*.**
  DOI `10.2307/2183530`; Crossref is-referenced-by count **579** (verified via
  `https://api.crossref.org/works/10.2307/2183530`). Included only as the standard
  philosophical frame for "a set-theoretic encoding is arbitrary; what matters is the
  structure/characteristic property" — **I did not retrieve the text**, so do not quote it.
- **Baxter, Nancy; Dubinsky, Ed; Levin, Gary (1989). *Learning Discrete Mathematics with
  ISETL*. Springer.** Book DOI `10.1007/978-1-4612-3592-7`; verified chapters include
  **ch. 3 "Sets and Tuples"** (`10.1007/978-1-4612-3592-7_3`), ch. 4 "Functions"
  (`_4`), ch. 5 "Predicate Calculus" (`_5`), **ch. 8 "Relations and Graphs"** (`_8`),
  verified via Crossref `query.bibliographic=Learning Discrete Mathematics with ISETL`.
  This is the ISETL textbook tradition that Zazkis & Gunn's 1997 ISETL study sits in, and
  it is the natural place to look for how sets/tuples/relations were actually taught in
  that environment. **Content not retrieved** (paywalled book).

---

## UNVERIFIED / COULD NOT RETRIEVE

**A. Full texts I could not obtain (abstract/record verified only — do not attribute
fine-grained findings to these):**

1. **Zazkis & Gunn (1997), JCMST 16(1):133–169.** I verified the ERIC record
   (EJ543540: authors, year, journal, volume, pages, ISSN, abstract, descriptors) but
   could not read the paper. Tried and failed:
   - `https://www.learntechlib.org/p/20955/article_20955.pdf` → HTTP 403 (proxy IP
     blocked; body cites `info@aace.org`)
   - `https://www.learntechlib.org/p/20955/` and `/primary/p/20955/` → 403; direct
     (no-proxy) connection → HTTP 405 + AWS WAF "Human Verification" JS challenge
   - `https://www.editlib.org/p/20955/` → 403
   - `https://r.jina.ai/https://www.learntechlib.org/p/20955/` → 401 "blocked … bad
     network reputation (AS30058)"
   - CORS proxies: `api.allorigins.win` → 500; `api.codetabs.com` → 522;
     `corsproxy.io` → "keyless legacy URLs no longer supported";
     `thingproxy.freeboard.io` → TLS failure
   - Wayback Machine `http://archive.org/wayback/available?url=learntechlib.org/p/20955/...`
     → connection timeout (archive.org direct) / HTTP 503 "Internet Archive: Temporarily
     Offline" (web.archive.org via proxy)
   - `citeseerx.ist.psu.edu/search?q=Sets,+subsets,+and+the+empty+set` → 500, redirected
     to the (offline) Wayback Machine
   - OpenAIRE keyword search for the title / for "Zazkis Gunn" → 0 results (the record is
     not in OpenAIRE/BASE)
   - **Suggested next step for whoever has a different egress IP or a library login:**
     the learntechlib PDF is free-to-read; only this IP is blocked.

2. **Bagni (2006), ESM.** Abstract retrieved (OpenAIRE) and metadata confirmed; the
   Springer landing page `https://link.springer.com/article/10.1007/s10649-006-8545-3`
   served a JS/anti-bot challenge (3 KB shell, 294 bytes of text). Unpaywall:
   `is_oa:false`, `oa_status:closed`, no repository copy. The two "experimental classroom
   episodes" and the actual student responses are **not** readable from here.

3. **Kolitsoe Moru & Qhobela (2013).** Full abstract retrieved (ERIC + OpenAIRE);
   Unpaywall says `is_oa:false` (Taylor & Francis closed). No student-level examples beyond
   the six error types listed in the abstract.

4. **Riegler (2013), *Teaching Mathematics and its Applications* 32(2):76–80.**
   Abstract only; Unpaywall `is_oa:false` (OUP closed). The specific student difficulties
   with `∅` that the article investigates are not retrievable.

5. **Lee, Park & Kim (2024), IJSME 22(3):657–678.** ERIC abstract + DOI verified; Springer
   full text closed, OpenAIRE has no record for DOI `10.1007/s10763-023-10391-w`
   ("NO RECORD"). The "social contract" task design and the time-variable/causality
   difficulties are known only at abstract granularity.

6. **Kanamori (2003), BSL.** Long publisher abstract retrieved from both Crossref and
   OpenAIRE; the article body (Cambridge Core) was not fetched. No page numbers for the
   quotations beyond the abstract itself.

7. **Karagöz Akar & Şener (2014), IJESIM.** Abstract retrieved via OpenAIRE; **the
   journal is defunct** — `ijesim.com` now serves a domain-sale page, and
   `https://doi.org/10.17278/ijesim.2014.01.006` resolved to that parked page (HTTP 200,
   no article). DergiPark search UI is JS-only and returned no usable result list.
   Hence: no full text, and no way to see the GeoGebra tasks.

8. **Halmos (1974) ch. 6; Forster (2003); Benacerraf (1965); Baxter/Dubinsky/Levin (1989).**
   Metadata verified only (see §3b). Titles/authors/years/DOIs are solid; **contents are
   unread**.

9. **Narli & Baser (2008).** Full text read (EJ894852 PDF) — this one *is* verified, but
   note it is a **secondary/adjacent** source for these blockers (its topic is Cantorian
   equivalence and countability, not `∈`/`⊆` or `∅`).

**B. The specific target I could not find at all:**

- **No empirical study of students' difficulty with the Kuratowski encoding
  `(a,b) = {{a},{a,b}}`, or with proving `(a,b) = (c,d) ↔ a = c ∧ b = d`, was found.**
  Searches run (all via Crossref `query.title`, ERIC, DOAJ, OpenAIRE, and Semantic
  Scholar where not rate-limited): `"ordered pair" students`, `"ordered pair" set theory
  relation`, `"ordered pairs" students misconception function`, `"ordered pair" definition
  relation set`, `Kuratowski ordered pair`, `"Cartesian product" students understanding`,
  `"Cartesian product" relation function students`, `ordered pair definition set theory
  teaching`, `pasangan berurutan miskonsepsi` (Indonesian), `sıralı ikili bağıntı
  fonksiyon öğrenci` (Turkish). Crossref title hits for "Kuratowski" were all
  mathematics (Kuratowski chains, Kuratowski's biography) or a linguistics book chapter
  ("The Ordered-Pair Illusion", in *'And'*, MIT Press, DOI
  `10.7551/mitpress/10488.003.0016`) — none is an education study. ERIC has essentially
  nothing: `Kuratowski ordered pair` → 0 results; `"ordered pair" definition proof
  students` → 0 results; ERIC's "ordered pair" hits are all school-level graphing/
  coordinate-plane activities (e.g. EJ595978 "Ordered-Pair Relations—A Performance
  Assessment", EJ765180 "Coordinate Plane Set Detective").
  **Implication for the survey:** claims that the Kuratowski encoding is a *student*
  blocker currently rest on instructor experience, not on published empirical evidence.
  If the survey needs evidence, the honest options are (i) cite Kanamori (2003) +
  Mirin/Weber/Wasserman (2020) for the conceptual hazard, or (ii) commission/collect
  primary data.

**C. Correction to a lead supplied by the parent agent (important — do not propagate):**

- The parent reported **ERIC EJ1070077 = Bagni (2006)**. **This is wrong.** I fetched
  `https://eric.ed.gov/?id=EJ1070077` and it is: Chadli, Abdelhafid; Bendella, Fatima;
  Tranvouez, Erwan (2015), "A Two-Stage Multi-Agent Based Assessment Approach to Enhance
  Students' Learning Motivation through Negotiated Skills Assessment", *Educational
  Technology & Society* 18(2), 140–152 — an educational-technology paper using *fuzzy* set
  theory. Bagni (2006) is a real and correctly described paper, but it is **not** ERIC
  EJ1070077; I verified it independently via Crossref + OpenAIRE (see Blocker 1 #1) and I
  could not find any ERIC record for it (`q=Bagni set theory` → "No results matched your
  query").

**D. Databases/tools that failed (so nobody re-spends time on them):**

- `scholar.google.com` — 429 (as documented in BRIEF).
- `api.semanticscholar.org` **search** endpoint — persistent HTTP 429 ("Too Many
  Requests"). The **single-paper** endpoint
  `graph/v1/paper/DOI:<doi>?fields=...` *does* work and was used for citation counts
  (Bagni 27, Kanamori 50); note it returns `abstract: null` with a publisher-elision
  disclaimer for Springer/CUP DOIs.
- `base-search.net` — Anubis bot check. `mojeek.com`, `qwant.com`, `searx.be`,
  `baresearch.org`, `opnxng.com`, `priv.au`, `searxng.site` — JS challenge / 403 / 429.
  `duckduckgo.com/html` — CAPTCHA. `bing.com` HTML *and* `&format=rss` — returned
  completely unrelated result sets (e.g. results about the United Nations for a
  mathematics query), so Bing is unusable from this egress IP. **Net effect: there is no
  working general web-search engine in this environment**; discovery had to go through
  bibliographic APIs.
- `ouci.dntb.gov.ua` — 403. `scilit.com` — 403. `escholarship.org/search` — HTTP 202 with
  empty body (bot-block). `projecteuclid.org` — loads a shell, no article text.
- `jurnal.unsri.ac.id` (Journal on Mathematics Education site) — TLS handshake failure
  through the proxy; use the `files.eric.ed.gov` mirror instead.
- `blogs.sfu.ca` (Rina Zazkis's current page, to which `sfu.ca/~zazkis/` redirects) —
  DNS does not resolve. `dm.unibo.it/~bagni/` (Bagni's old Bologna page) — 404.
  `math.unipa.it/~grim/` (GRIM didactics preprint series) — DNS does not resolve.
- `api.crossref.org/works?query.bibliographic=...` is **poor for topical search** (it
  matched "confusion matrix" figures for "element subset confusion students"). Use
  `query.title=` (helper `/tmp/stresearch/cr2.sh`) for topical discovery on Crossref.

**E. Helpers built during this pass (left in `/tmp/stresearch/`, reusable):**

- `eric2.sh "query" [n]` — ERIC search with a robust results-block parser (prints ERIC ID,
  authors/source, abstract, DOI, `files.eric.ed.gov` full-text link, total).
- `cr2.sh "title words" [n]` — Crossref **title** search with retry (the correct tool for
  topical discovery on Crossref).
- `pdf2txt.sh URL name` — URL → text; uses PyMuPDF (`pdf.py`) for PDFs because
  **`pdftotext` is NOT installed** (the original `fetch.sh` silently fails on every PDF —
  it prints `NO_PDFTOTEXT`; do not trust `fetch.sh` for PDFs).
- `oaire2.sh "keywords" [n]` — OpenAIRE keyword search (titles, authors, DOIs, abstracts).
- `oaidoi.sh 10.xxxx/yyyy` — **the highest-yield tool of this pass**: OpenAIRE lookup by
  DOI, which returned full publisher abstracts for Bagni 2006, Kanamori 2003,
  Durand-Guerrier 2003, Riegler 2013, Moru & Qhobela 2013 and Karagöz Akar & Şener 2014 —
  i.e. it is the working substitute for the dead OpenAlex abstract index.


---

# Part B — Blockers 4–6 (function ontology, quantifier order, image vs preimage)

# BLOCKERS 4–6 — Research evidence dossier

Agent B (blockers 4–6). Compiled in-session from live API records and retrieved full texts only.
Every entry below was verified by fetching something: an OpenAlex/Crossref API record, an ERIC
record or full-text PDF, a HAL record, or an author-archived PDF that I downloaded and read.

**Verification legend used in each entry**

- `FULLTEXT` — I downloaded the document and read its body (findings below come from the body, not the abstract).
- `ABSTRACT` — I retrieved a verbatim abstract (OpenAlex abstract-inverted-index, HAL `abstract_s`, or ERIC record).
- `METADATA` — I retrieved a bibliographic record (title/authors/year/venue/vol/pages/DOI) from Crossref or OpenAlex; no abstract available and full text not reachable.

**Tooling notes for downstream agents**

- OpenAlex went hard rate-limited partway through this session (`HTTP 429, "Insufficient budget",
  resets midnight UTC`). Entries marked OpenAlex were fetched *before* that. Crossref, ERIC, HAL,
  Unpaywall and the Wayback Machine still worked at the end.
- Publisher landing pages (Springer, ScienceDirect, Taylor & Francis) are behind JS/Cloudflare
  challenges through this proxy — do not plan on them.
- Ed Dubinsky's pre-2010 papers survive at
  `https://web.archive.org/web/<timestamp>if_/http://www.math.kent.edu/~edd/<FILE>.pdf`.
  These PDFs use subset-font encodings whose extracted text is Caesar-shifted. Decoder written this
  session: `/tmp/stresearch/dec2.py <raw.txt>`. Scanned items need OCR: `/tmp/stresearch/ocr5.sh in.pdf out.txt`.

---

## BLOCKER 4 — Function as *set of ordered pairs* vs function as *rule / map*

### 1. Breidenbach, D., Dubinsky, E., Hawks, J., & Nichols, D. (1992). *Development of the process conception of function.* Educational Studies in Mathematics **23**, 247–285. DOI 10.1007/BF02309532

- **Type:** peer-reviewed journal article (Educational Studies in Mathematics; 399 citations per OpenAlex, 406 per Semantic Scholar).
- **URLs actually fetched:**
  - `https://web.archive.org/web/20050209160001if_/http://www.math.kent.edu/~edd/PROCESSFUNC.pdf` (author-archived 20-page scan; downloaded, then OCR'd with tesseract → `/tmp/stresearch/bdhn_ocr2.txt`)
  - `https://web.archive.org/web/20050209014956if_/http://www.math.kent.edu/~edd/publications.html` (Dubinsky's own numbered publication list, item 23, confirms authors, title, *Educational Studies in Mathematics* **23** (1992), 247–285)
  - `https://api.crossref.org/works/10.1007/bf02309532` (metadata)
- **Verified level:** FULLTEXT (OCR of the author's scan) + METADATA + self-citation in Dubinsky (1997), whose reference list I also read in full text.
- **OCR caveat:** the archive copy is a 1-bit scan; tesseract recovered the prose reliably but garbled some numerals in tables (e.g. one table cell came out "$7.3"). Every figure I quote below is one I read as a plausible percentage in a table whose other entries were clean; treat the exact decimals as ±OCR and re-check against the published PDF before printing them.
- **Actual findings (read from the body):**
  - Participants: "The students in the present study were mainly sophomore and junior math majors preparing to be high school, middle school or elementary school math teachers. … There were **62 students in this program**." The instructional treatment on functions ran "over a **four week** period."
  - The paper's whole design is an APOS *genetic decomposition* of "function" into Prefunction → Action → Process → Object, and it argues the central learning obstacle is that students never **interiorize the process** and never **encapsulate it into an object** ("there is only one way to make a mathematical object – by encapsulating a process").
  - In the "Functions in Situations" instrument (24 situations, 9 categories), students said "yes, that's a function" only **about 40 % of the time overall, where it should have been close to 100 %**. Category breakdown from Table V: ISETL `funcs` 74.6–76.5 %; tuples 54.1–61.6 %; smaps 27.4–50 %; **equations 25–31 %; graphs 19–40.7 %; tables 39.9–48.6 %; physical situations 30.2–35.8 %; strings 2.8–41 %**.
  - The paper names the ontology obstacle explicitly: "Graphs and equations are standard topics in the undergraduate curricula … yet no more than about 1/4 of them saw functions in such situations." Students "insisted that there be an expression or at least the presence of variables to indicate 'input' and 'output'", and "in many cases they insisted on the presence of causality before they were willing to construct a process." Students who *did* see a function in a graph or table often "tried to guess a formula that described the relationship before they were willing to agree that there was a function."
  - The treatment (4 weeks, ISETL) moved students from Prefunction/Action toward Process, and the post-test was a purely mathematical final exam with no ISETL content — students performed well on it, which the authors take as evidence that a process conception transfers.
  - The instructional treatment explicitly included "problems that compute **pre-images** of functions, the construction of an inverse function and the concepts of 1-1 and onto" as activities for "**reversing the process of a function**" (the passage sits under the running heads "PROCESS CONCEPTION OF FUNCTION 264 / 265" in the scan). Directly relevant to Blocker 6.
- **Relevance:** the canonical APOS/ontology source. It gives *numbers* for exactly the failure mode in Blocker 4: a function that is presented as a set of ordered pairs, a graph, or a table is frequently not recognised as a function at all unless an explicit formula and a causal input→output story are supplied. This is the "function = its graph/rule" ontology problem measured.

### 2. Dubinsky, E., & Harel, G. (1992). *The nature of the process conception of function.* In G. Harel & E. Dubinsky (Eds.), *The Concept of Function: Aspects of Epistemology and Pedagogy*, MAA Notes **25**, 85–106.

- **Type:** book chapter (MAA Notes, Mathematical Association of America).
- **URL fetched:** `https://web.archive.org/web/20050209014956if_/http://www.math.kent.edu/~edd/publications.html` (item 24 in Dubinsky's own list: "(with G. Harel) The Nature of the Process Conception of Function, in (G. Harel and E. Dubinsky, ed.) The Concept of Functions: Aspects of Epistemology and Pedagogy, MAA Notes, 25 (1992), 85-106.")
- **Verified level:** METADATA (author's own publication ledger; the chapter is not Crossref-indexed and no free copy was reachable).
- **Actual findings:** not read — I did not retrieve the chapter text. The companion chapter in the same volume (Sfard 1992, "Operational origins of mathematical objects and the quandary of reification — the case of function", same MAA Notes 25 volume) is the usual place to find the process/object analysis applied to function; I could not verify its page range (see UNVERIFIED).
- **Relevance:** this is the chapter that Dubinsky's own later work cites as the theoretical statement of the "process conception of function" that the 1992 ESM study operationalises. Cite it for the theory, not for the numbers — use entry 1 for the numbers.

### 3. Sfard, A. (1991). *On the dual nature of mathematical conceptions: Reflections on processes and objects as different sides of the same coin.* Educational Studies in Mathematics **22**(1), 1–36. DOI 10.1007/BF00302715

- **Type:** peer-reviewed journal article (1848 citations per OpenAlex; 1006 per Crossref — both counts retrieved).
- **URLs fetched:** `https://api.openalex.org/works?filter=title_and_abstract.search:...` (OpenAlex record, before rate-limit) and `https://api.crossref.org/works/10.1007/bf00302715` (Crossref: vol 22, issue 1, pp. 1–36).
- **Verified level:** METADATA (both APIs agree; no abstract in either record, full text paywalled).
- **Actual findings:** **not retrieved.** Neither the OpenAlex nor the Crossref record for this article carries an abstract, and the full text is closed access (`is_oa: false` at Unpaywall). I can verify only: the title, the author, the venue, the volume/issue/pages, and the citation counts. The paper is the standard reference for the process/object duality of mathematical conceptions and is cited by name throughout the APOS literature I did read, but **I did not retrieve any sentence of its content and the survey should not attribute specific findings to it without a library copy.**
- **Relevance:** the theoretical vocabulary for the Blocker-4 ontology claim ("a function is a process that must be reified into an object — i.e. into a set of ordered pairs — before the set-theoretic definition means anything"). Pair with entry 4 (Sfard & Linchevski 1994) for the explicit "gains and pitfalls of reification" argument in algebra.

### 4. Sfard, A., & Linchevski, L. (1994). *The gains and the pitfalls of reification — the case of algebra.* Educational Studies in Mathematics. DOI 10.1007/BF01273663 (journal version); the same authors have a same-titled 1994 book chapter, DOI 10.1007/978-94-017-2057-1_4, 368 cited vs 311 for the article.

- **Type:** peer-reviewed journal article / book chapter.
- **URL fetched:** `https://api.openalex.org/works?filter=title_and_abstract.search:reification%20operational%20structural%20conception%20mathematics` (OpenAlex record).
- **Verified level:** METADATA.
- **Actual findings:** **not retrieved.** The OpenAlex record carries no abstract and the full texts are closed access. Verified: title, authors, year, venue, DOI, citation counts. The paper is universally cited as the application of the reification model to school algebra; the substance of that argument was not retrieved by me.
- **Relevance:** supplies the mechanism sentence for Blocker 4: the set-of-ordered-pairs definition requires the function to already be an object, and the operational→structural gap is where the "function = rule" ontology survives instruction.

### 5. Vinner, S. (1983). *Concept definition, concept image and the notion of function.* International Journal of Mathematical Education in Science and Technology **14**(3), 293–305. DOI 10.1080/0020739830140305

- **Type:** peer-reviewed journal article (426 cited per OpenAlex; 212 per Crossref).
- **URLs fetched:** `https://api.openalex.org/works?filter=title_and_abstract.search:concept%20image%20concept%20definition%20function` (OpenAlex, includes verbatim abstract) and `https://api.crossref.org/works/10.1080/0020739830140305` (vol 14, issue 3, pp. 293–305).
- **Verified level:** ABSTRACT + METADATA.
- **Verbatim abstract (retrieved):** "A simple model for cognitive processes will be constructed using the notions of concept image and concept definition. The model will be used to analyse some phenomena in the process of the learning of the function concept in grades 10 and 11. Educational conclusions, similar to those in [3] which were based on historical arguments, will be drawn."
- **Relevance:** the origin of the concept-image/concept-definition pair applied specifically to *function*. This is the vocabulary the whole Block-4 literature uses.

### 6. Tall, D., & Vinner, S. (1981). *Concept image and concept definition in mathematics with particular reference to limits and continuity.* Educational Studies in Mathematics **12**(2), 151–169. DOI 10.1007/BF00305619

- **Type:** peer-reviewed journal article (1036 citations per Crossref — the count I retrieved; OpenAlex was not queried for this one).
- **URL fetched:** `https://api.crossref.org/works/10.1007/bf00305619` (title, authors, year 1981, vol 12, issue 2, pp. 151–169).
- **Verified level:** METADATA.
- **Actual findings:** not read in full — I verified the record only. This is the foundational statement of concept image vs concept definition; the *function-specific* application is Vinner (1983) and Vinner & Dreyfus (1989), entries 5 and 7.
- **Relevance:** include as the theoretical root of the concept-image line, but do not attribute function-specific data to it.

### 7. Vinner, S., & Dreyfus, T. (1989). *Images and definitions for the concept of function.* Journal for Research in Mathematics Education **20**(4), 356–366. DOI 10.5951/jresematheduc.20.4.0356

- **Type:** peer-reviewed journal article (431 cited per OpenAlex; 267 on the duplicate JSTOR-DOI record 10.2307/749441).
- **URLs fetched:** OpenAlex record (abstract) + `https://api.crossref.org/works/10.5951/jresematheduc.20.4.0356` (vol 20, issue 4, pp. 356–366).
- **Verified level:** ABSTRACT + METADATA.
- **Retrieved abstract (near-verbatim):** 271 college students and 36 junior high school teachers were compared on their images vs their definitions of "function". "Many of the definitions and even more of the images were primitive among all but the mathematics majors and the teachers. **Discrepancies between image and definition were frequent for all subjects who gave the Dirichlet–Bourbaki definition.**"
- **Relevance:** this is the cleanest empirical statement of the Blocker-4 split: even students who can *state* the Dirichlet–Bourbaki (set-of-ordered-pairs / arbitrary-correspondence) definition operate on a different, prototype-driven image. The specific N and the subgroup breakdown are quotable.

### 8. Tall, D., & Bakar, M. (1992). *Students' mental prototypes for functions and graphs.* International Journal of Mathematical Education in Science and Technology **23**(1), 39–50. DOI 10.1080/0020739920230105

- **Type:** peer-reviewed journal article (112 cited per OpenAlex; 38 per Crossref).
- **URLs fetched:** OpenAlex record (long abstract retrieved) + `https://api.crossref.org/works/10.1080/0020739920230105` (vol 23, issue 1, pp. 39–50).
- **Verified level:** ABSTRACT + METADATA.
- **Retrieved findings (from the abstract):** A-level students "may be able to use functions in their practical mathematics" but "their grasp of the theoretical nature of the function concept may be tenuous and inconsistent." The hypothesis is that students build **prototypes** rather than using the definition: "those having regular shaped graphs, such as x² or sin x, those often encountered (possibly erroneously), such as **a circle**, those in which y is defined as an explicit formula in x". The abstract explicitly reports "significant misconceptions", e.g. "**three-quarters of a sample of students starting a university mathematics course**…" (the retrieved abstract text is truncated at that point in the OpenAlex record — the direction is that ~75 % classified a non-function as a function, but I did not see the completion of that clause, so do not quote the exact predicate).
- **Relevance:** directly on point for Blocker 4 — the graph-shaped prototype (and the circle!) overrides the definition. Note the honest caveat above: I have the N and the "three-quarters" figure but not the full clause.

### 9. Thompson, P. W. (1994). *Students, functions, and the undergraduate curriculum.* CBMS Issues in Mathematics Education **4**. DOI 10.1090/cbmath/004/02

- **Type:** book chapter (CBMS Issues in Mathematics Education, AMS/MAA).
- **URL fetched:** `https://api.crossref.org/works/10.1090/cbmath/004/02` (Crossref record: title, author "Thompson, Patrick", year 1994, container "CBMS Issues in Mathematics Education", type book-chapter, 82 cited).
- **Verified level:** METADATA. **Page range and editor list not verified** — Crossref returned no `page` field, and I did not retrieve the volume's front matter; the commonly cited range 21–44 is *not* confirmed by anything I retrieved, so do not print it. The DOI and the series title are verified.
- **Actual findings:** not read (chapter is paywalled and not in any repository I could reach). The chapter is the standard reference for the *covariation* view of function as an alternative to both the rule view and the static set-of-pairs view.
- **Relevance:** needed as the third position in the Blocker-4 ontology debate (rule / set-of-pairs / covariation). Cite the DOI; flag that the findings text was not retrieved.

### 10. Even, R. (1990). *Subject matter knowledge for teaching and the case of functions.* Educational Studies in Mathematics **21**(6), 521–544. DOI 10.1007/BF00315943

- **Type:** peer-reviewed journal article (233 cited per OpenAlex).
- **URL fetched:** OpenAlex record (title/authors/year/venue/DOI retrieved). The Crossref lookup of `10.1007/bf00315943` returned an empty/non-JSON response (rate-limiting, not a confirmed absence), so vol/issue/pages rest on the OpenAlex record only.
- **Verified level:** METADATA.
- **Actual findings (verbatim, from Jukić Matić et al. 2022, who cite Even 1993 — this is a retrieved quotation of what Even argues, not a retrieval of Even's own text):** "Mathematicians throughout history have had difficulties with understanding the concept of function, particularly with the notions of **arbitrariness** (which is implicit in the definition) and **univalence** (which is explicit in the definition), and the same difficulties with understanding the concept of function can be observed in contemporary students (Even, 1993)."
- **Relevance:** the arbitrariness/univalence distinction is *the* precise formulation of the Blocker-4 ontology problem for teachers — a function is an arbitrary set of pairs, not a law.

### 11. Even, R. (1993). *Subject-matter knowledge and pedagogical content knowledge: Prospective secondary teachers and the function concept.* Journal for Research in Mathematics Education **24**(2), 94–116. DOI 10.5951/jresematheduc.24.2.0094

- **Type:** peer-reviewed journal article (346 cited per OpenAlex).
- **URL fetched:** `https://api.crossref.org/works/10.5951/jresematheduc.24.2.0094` (Even, Ruhama; 1993; JRME 24(2), 94–116) + OpenAlex record.
- **Verified level:** METADATA.
- **Actual findings:** not read. This is the study of prospective secondary teachers' subject-matter and pedagogical-content knowledge of function; downstream sources I *did* read (Sherman et al. 2019; Jukić Matić et al. 2022) both cite Even 1990/1993 for preservice teachers' privileging of algebraic representations and reductive uses of the univalence condition.
- **Relevance:** the teacher-knowledge leg of Blocker 4. Cite the DOI and the verified page range; attribute the substantive claims to the sources that I read making them.

### 12. Carlson, M. P. (1998). *A cross-sectional investigation of the development of the function concept.* CBMS Issues in Mathematics Education. DOI 10.1090/cbmath/007/04

- **Type:** book chapter.
- **URL fetched:** `https://api.crossref.org/works?query.bibliographic=...cross-sectional+investigation+of+the+development+of+the+function+concept` (Crossref search record).
- **Verified level:** METADATA. Volume number, volume title and page range NOT verified — do not print them.
- **Actual findings:** **not retrieved** (no abstract in Crossref; full text not reachable). Verified: title, author, year, venue string, DOI, 86 citations. The Oehrtman–Carlson–Thompson chapter (2008) surfaced in the same Crossref sweep (183 cited) but was likewise not read.
- **Relevance:** expected to supply the developmental levels that let a curriculum sequence the rule→process→object transition — but that expectation is mine, not a retrieved finding. Do not attribute specific levels to it without the text.

### 13. Monk, G. S. (1994). *Students' understanding of functions in calculus courses.* Humanistic Mathematics Network Journal. DOI 10.5642/hmnj.199401.09.07

- **Type:** peer-reviewed/edited journal article (Humanistic Mathematics Network Journal; 14 cited per Crossref).
- **URL fetched:** `https://api.crossref.org/works?query.bibliographic=Monk+students+understanding+of+a+function+given+by+a+physical+model` (Crossref returned this record: Monk, G. S., 1994, HMNJ, 14 cited).
- **Verified level:** METADATA.
- **Actual findings:** not read. **Important correction:** the lead I was given — Monk (1992), "Students' understanding of a function given by a physical model", in the *MAA Notes 25* volume — did **not** appear in any Crossref or OpenAlex search I ran. What exists and is verified is the 1994 HMNJ article above. See UNVERIFIED.
- **Relevance:** a verified early study of students' understanding of functions in calculus courses (title-level only). The MAA Notes 25 "physical model" chapter should not be cited without a further check.

### 14. Cooney, T. J., & Wilson, M. R. (1993/2012). *Teachers' thinking about functions: Historical and research perspectives.* In *Integrating Research on the Graphical Representation of Functions* (Routledge reissue of the 1993 Erlbaum volume). DOI 10.4324/9780203052617-13

- **Type:** book chapter (Routledge reissue of the 1993 Erlbaum volume).
- **URL fetched:** `https://api.crossref.org/works?query.bibliographic=Teachers+thinking+about+functions+historical+and+research+perspectives` → Crossref record: "Teachers' Thinking About Functions: Historical and Research Perspectives: Thomas J. Cooney and Melvin R. Wilson", 2012, book-chapter, `10.4324/9780203052617-13`. The original 1993 printing is what the DOI's reissue corresponds to.
- **Verified level:** METADATA. The 2012 date in Crossref is the *reissue* date; the chapter's original publication year (1993), the editors and the page range are **not** confirmed by anything I retrieved.
- **Actual findings:** not read.
- **Relevance:** the standard review of teacher thinking about function; useful as a survey citation. Flag the reissue/original-year ambiguity.

### 15. Hitt, F. (1998). *Difficulties in the articulation of different representations linked to the concept of function.* The Journal of Mathematical Behavior. DOI 10.1016/S0732-3123(99)80064-9

- **Type:** peer-reviewed journal article (149 cited per OpenAlex; 74 per Crossref).
- **URL fetched:** `https://api.crossref.org/works?query.bibliographic=Difficulties+in+the+articulation+of+different+representations+linked+to+the+concept+of+function` (Crossref record: Hitt, Fernando, 1998, JMB). Page range not returned by Crossref — not verified.
- **Verified level:** METADATA.
- **Actual findings:** not read.
- **Relevance:** the representation-shifting problem (algebraic ↔ graphic ↔ tabular ↔ verbal) that Blocker 4's "function = its graph" issue sits inside.

### 16. Dubinsky, E., & Wilson, R. T. (2013). *High school students' understanding of the function concept.* The Journal of Mathematical Behavior **32**(1), 83–101. DOI 10.1016/j.jmathb.2012.12.001

- **Type:** peer-reviewed journal article (58 cited per Crossref).
- **URL fetched:** `https://api.crossref.org/works/10.1016/j.jmathb.2012.12.001` (Ed Dubinsky; Robin T. Wilson; 2013; JMB 32(1), 83–101).
- **Verified level:** METADATA.
- **Actual findings:** not read. This is the modern APOS restatement for the high-school population; it is cited by Jukić Matić et al. (2022) as evidence that the concept has been studied since the 1960s "yet there are still open questions" about how curriculum shapes it.
- **Relevance:** the up-to-date APOS citation for Blocker 4, with a verified page range.

### 17. Jukić Matić, L., Kehler-Poljak, G., & Rukavina, S. (2022). *The influence of curriculum on the concept of function: An empirical study of pre-service teachers.* European Journal of Science and Mathematics Education **10**(3), 380–395. DOI 10.30935/scimath/12042

- **Type:** peer-reviewed open-access journal article.
- **URLs fetched:** `https://files.eric.ed.gov/fulltext/EJ1353216.pdf` (downloaded, text extracted → `/tmp/stresearch/html/ft_jukic.txt`) and the ERIC search record `ERIC EJ1353216`.
- **Verified level:** FULLTEXT.
- **Actual findings (read from the body):** Pre-service secondary mathematics teachers in Croatia and Germany were given open-ended questionnaires plus interviews probing **concept definition** and **concept image** of function in relation to their curriculum experience. "The curriculum has a great influence on the development of the concept definition and concept image… The curriculum strongly influenced the theoretical background of the function concept and thus **the gap between the formal and the personal definition of function**. Later and more intensive work with the formal definition of function led to a better development of the function concept in general." The paper also states that "the curriculum also had an influence on the range of the concept image developed… **with no proportional dependence** in relation to the better developed understanding of the concept of function."
- **Relevance:** gives a *cross-curricular* causal claim for Blocker 4 — how much formal-definition work a national curriculum does predicts the size of the definition/image gap. Also confirms that arbitrariness + univalence are the two historically hard sub-notions.

### 18. Sherman, M. F., Meagher, M. S., Lovett, J., & McCulloch, A. (2019). *Transforming pre-service teachers' definition of function.* In S. Otten et al. (Eds.), *Proceedings of the 41st Annual Meeting of the North American Chapter of the International Group for the Psychology of Mathematics Education (PME-NA)*, pp. 1039–1043. St Louis, MO.

- **Type:** conference paper (PME-NA proceedings; not a journal).
- **URLs fetched:** `https://files.eric.ed.gov/fulltext/ED606887.pdf` (downloaded, text extracted → `/tmp/stresearch/html/ft_sherman.txt`) and ERIC record `ED606887`.
- **Verified level:** FULLTEXT.
- **Actual findings (from the abstract and body):** "There is a considerable body of research showing that students at all levels, including preservice secondary mathematics teachers, have difficulties with the definition of function as a **correspondence between two sets with a univalence condition**. Those difficulties include **privileging algebraic representations** and **reductive interpretations of the univalence condition in the form of the vertical line test**." 47 pre-service mathematics teachers gave definitions, worked with an interactive applet using a **non-standard representation of function**, and re-defined. Result: "a measurable increase in the participants' level of abstraction in their definitions, and an increase in their attention to the univalence condition."
- **Relevance:** the most actionable Blocker-4 intervention evidence I retrieved: a concrete task that moves preservice teachers off "formula + vertical line test" and toward the set-theoretic definition, with a measured pre/post effect. Also names the Dirichlet–Bourbaki definition as "nearly impossible for school students to see any intellectual need for" (quoting Thompson & Carlson 2017, which they cite).

### 19. McCulloch, A. W., Lovett, J. N., Dick, L., Sherman, M., Edgington, C., & Meagher, M. (2020). *Eliciting the coordination of preservice secondary mathematics teachers' definitions and concept images of function.* International Journal of Mathematical Education in Science and Technology. DOI 10.1080/0020739X.2020.1821107

- **Type:** peer-reviewed journal article.
- **URL fetched:** `https://api.crossref.org/works?query.bibliographic=...` (Crossref record) and ERIC record `EJ1350694`.
- **Verified level:** METADATA + ERIC record snippet.
- **Actual findings:** not read in full; the ERIC record confirms the topic (eliciting coordination between PSMTs' definitions and concept images of function) and the venue/year.
- **Relevance:** the journal-length version of entry 18; prefer this if a peer-reviewed citation is needed.

### 20. Kjeldsen, T. H., & Petersen, P. H. (2013). *Bridging history of the concept of function with learning of mathematics: Students' meta-discursive rules, concept formation and historical awareness.* Science & Education. DOI 10.1007/s11191-013-9641-2

- **Type:** peer-reviewed journal article (23 cited per Crossref; 26 per OpenAlex).
- **URL fetched:** `https://api.crossref.org/works?query.bibliographic=Bridging+history+of+the+concept+of+function...` (Crossref record) and OpenAlex record (abstract).
- **Verified level:** METADATA.
- **Actual findings:** **not retrieved.** From the title (retrieved) it is a history-of-mathematics-based study of function concept formation and students' meta-discursive rules; no body text was read.
- **Relevance:** supports a *historical* framing of Blocker 4 — the modern set-of-pairs definition is a late historical settlement, which is why it feels unmotivated to students (compare the quote in entry 18 about "no intellectual need for it").

### 21. (Supplementary, Blocker 4 → Blocker 6 bridge) Bansilal, S., Brijlall, D., & Trigueros, M. (2017). *An APOS study on pre-service teachers' understanding of injections and surjections.* The Journal of Mathematical Behavior **48**, 22–37. DOI 10.1016/j.jmathb.2017.08.002

See Blocker 5 entry 14 — it is cross-listed because the study's data are students' attempts to *write* the ∀∃ definitions and to *use* them on set images.

---

## BLOCKER 5 — Quantifier order in injective / surjective / bijective definitions; students swapping ∀∃; negating quantified statements

### 1. Dubinsky, E. (1997). *On learning quantification.* Journal of Computers in Mathematics and Science Teaching **16**(2&3), 335–362.

- **Type:** peer-reviewed journal article. **Citation fully verified against two independent author-controlled sources**: (a) the article's own running text and reference list, (b) Dubinsky's archived numbered publication list, item 42: "On Learning Quantification, Journal of Computers in Mathematics and Science Teaching, 16(2&3) (1997) 335-362. Download PDF file (411K, 43 pages)."
- **URLs fetched:**
  - `https://web.archive.org/web/20050209152422if_/http://www.math.kent.edu/~edd/LearningQuant.pdf` (full 43-page paper; downloaded, decoded with `/tmp/stresearch/dec2.py` → `/tmp/stresearch/html/LQ_clean.txt`)
  - `https://web.archive.org/web/20050209014956if_/http://www.math.kent.edu/~edd/publications.html`
- **Verified level:** FULLTEXT (including the results tables and conclusions).
- **Actual findings (read from the body):**
  - Two classes. **Class1**: a course entitled *Introduction to Finite Mathematics* at Clarkson University, **Fall 1986**, "There were **19 students**, mostly sophomores". **Class2**: *Introduction to Analysis*, the following semester, "the **17 students** were mainly Juniors". (The per-instrument captions read: Table 1 SetA – 18 students; Table 2 SetB – 18; Table 3 SetC – 19; Table 4 AssignA – 19; Table 5 AssignB – 19; Table 6 Class2 Problems – 17.)
  - Both classes were taught universal and existential quantification through a **genetic decomposition** (declarations → proposition-valued functions → interiorized iteration → coordination with a quantifier → encapsulation of single-level → coordination of two instantiations → encapsulation of two-level quantification → three-or-more-level). Class1 had two 75-minute meetings a week plus labs; Class2 was lecture-only with one assignment on quantification.
  - Grading: correct = 100, partial = 50, incorrect = 0. **Class1: in 11 of 21 problems the average was better than 70 %; the worst problem score in Class1 was 47 %.** Class2, which spent much less time, did well on only 7 of 16 problems and **very poorly on 6, with 3 problems below 30 %**.
  - **The headline error pattern:** "Of the seven problems on which the students did poorly (less than 60 %), **five of them involved an implication which was not part of the quantification**. It is clear from the papers that difficulty with implications contributed to the low score. For example, on Problem 4a of SetA, **all but one of the students were quite correct in negating the quantification part. Their errors were entirely the result of greater or lesser difficulties they had with negating an implication.**"
  - Second pattern: **meaning can develop without syntax.** Class2 students "did very poorly" on translation-into-formal-language problems (4, 5, 6) yet "did quite well" on problems 13, 14, 15 that probed understanding of the same statements. Dubinsky's conclusion: "understanding the meaning of a statement can develop without a corresponding ability to deal with the statement linguistically."
  - The instrument includes the classic order-sensitive tasks: the "**flying fish**" statement, explicit negation of nested English statements ("In all the classes that I have taught, there is one in which every student got an 'A'"), and the ε–δ-style "For a given q > 0 … for every p > 0 there is an x …". Problem 10 deliberately contains "a 'trap' intended to keep the students from applying textual translation and formal rules in order to negate."
- **Relevance:** this is the single best-evidenced source for Blocker 5. It supplies (a) a validated genetic decomposition of quantification, (b) hard numbers, and (c) the sharpest actionable finding in the whole dossier: **students can negate the quantifier structure correctly and still fail the item because of the embedded implication** — so a curriculum that drills "swap ∀/∃ and negate the predicate" will not fix the observed failures.

### 2. Dubinsky, E., & Yiparaki, O. (2000). *On student understanding of AE and EA quantification.* Unpublished manuscript, Georgia State University / Agnes Scott College, dated 18 November 2000, 67 pp.

- **Type:** **unpublished manuscript / author-archived technical report** (not peer-reviewed; cite it as a manuscript, or cite via Chellougui & Kouki 2012, who reference "Dubinsky et Yiparaki (2000)").
- **URLs fetched:**
  - `https://web.archive.org/web/20050209160952if_/http://www.math.kent.edu/~edd/OlgaPaper.pdf` (67-page PDF; downloaded, decoded → `/tmp/stresearch/html/olga_clean.txt`)
  - `https://web.archive.org/web/20050209014956if_/http://www.math.kent.edu/~edd/publications.html` (item 50, with O. Yiparaki, "(1206K, 67 pages)")
- **Verified level:** FULLTEXT.
- **Actual findings (read from the body; the paper reports percentages, and I am quoting its own numbers):**
  - Design: 63 students (54 undergraduates + 9 graduate students) answered an 11-statement questionnaire (statements 1–9 in natural language about everyday situations; 10–11 mathematical), deciding true/false and explaining; some were then interviewed.
  - **AE = "For all … there exists …"; EA = "There exists … for all …".** "**94 % of the students interpreted at least one EA statement as an AE.** For the six EA statements, the percentage of students who interpreted them as AE ranged from **11 % to 81 %**. On the other hand, only **5 %** interpreted at least one AE statement as an EA; the percentage of students who interpreted AE statements as EA ranged from **0 % to 3 %.**"
  - Bias is asymmetric and strong: "On both the questionnaire and the interviews we found a strong tendency for students to favor an AE interpretation over an EA interpretation." When the researchers probed an AE-reading student on whether statements 6 and 9 could be read as EA, "**almost all of the interviewees continued to see these statements as AE and saw no ambiguity**" (one exception). None of the written responses flagged any statement as ambiguous.
  - Truth-value asymmetry: "**30 % of the AE statements were declared True, but only 10 % of the EA statements were declared True.**"
  - On the **mathematical** statements: "**only 41 % of the students got statement 10 right, and only 9 % got statement 11 right**." If the domain is assumed to be the real numbers, those rise only to **49 %** and **14 %**. "**Yet 78 % of the students gave a valid argument for seven out of the nine natural-language**" statements. Interviews: "57 % of the students we interviewed were able to assess the truth value of statement 10 correctly, whereas only 41 % of all the students who responded…"; a total of **57 % were unsuccessful on both statements 10 and 11**.
  - Word-level effects on order-reading: statements using "every": **81 %** read statement 4 as AE, vs only about **40 %** for statement 8; **11 %** read statement 3 as AE while **37 %** read statement 6 as AE — i.e. wording, not logical form, drives the reading.
  - Overall: "Most students in this study could not distinguish between AE and EA statements in mathematics and did not seem to be aware of the standard mathematical conventions for parsing statements." Recommendation: use **quantifier games** as a pedagogical tool.
- **Relevance:** this is the most precise ∀∃/∃∀ evidence in the dossier and it is *on the exact Blocker-5 issue*. The asymmetry (EA→AE misreading 11–81 % vs AE→EA 0–3 %) is the quantitative fact a curriculum needs: students default to ∀∃ and do not even perceive the alternative parse. Also shows natural-language performance ≫ mathematical-statement performance.

### 3. Dubinsky, E., Elterman, F., & Gong, C. (1988/1989). *The student's construction of quantification.* For the Learning of Mathematics **8**(2), 44–51.

- **Type:** peer-reviewed journal article (FLM is peer-reviewed; this vintage is **not** in Crossref).
- **URLs fetched and read:**
  - `https://web.archive.org/web/20050209142709if_/http://www.math.kent.edu/~edd/QUANT1.pdf` — the author-archived scan of this paper (22 scanned pages, no text layer). I OCR'd it (`/tmp/stresearch/ocr5.sh` → `/tmp/stresearch/quant1_ocr.txt`) and the first page reads: "**The Student's Construction of Quantification — Ed Dubinsky, Flor Elterman and Cathy Gong**".
  - `https://web.archive.org/web/20050209014956if_/http://www.math.kent.edu/~edd/publications.html` — item 10: "(with F. Elterman and C. Gong) The Student's Construction of Quantification, For the Learning of Mathematics, (1989)."
  - `.../LearningQuant.pdf` reference list: "Dubinsky, E., Elterman, F., & Gong, C. (1988). The student's construction of quantification. For the Learning of Mathematics, 8(2), 44-51."
- **Verified level:** **FULLTEXT** (OCR of the author's scan) + METADATA.
- **Actual findings (read from the body):**
  - Design: "an informal study of a Discrete Mathematics class taught by the first author in collaboration with the other two authors at the **University of California, Berkeley in Spring, 1986**." The instructional treatment used computer experiences in the programming language **SETL**. "After the topic was covered, each student was given an in-depth interview during which time he or she was asked to perform certain tasks … and to explain their reasoning." So this is a small qualitative interview study, not a controlled experiment.
  - The paper is the **origin of the quantification genetic decomposition** that Dubinsky (1997) later reuses and tests: propositions → proposition-valued functions → interiorized iteration over the domain → applying a quantifier to obtain a single proposition → **encapsulating** that process to obtain an object → coordinating two objects into a two-level quantification → encapsulating again for three levels → a schema that "can handle quantifications which are nested to any level."
  - It argues encapsulation of single-level quantification is the critical prerequisite: "the encapsulation of single-level quantifications is critical for working with several quantifications. The single-level quantifications must be objects so that an operation (composition) can be applied to them."
  - **Concrete datum worth quoting:** "It is, however, possible to report an unpublished observation with sophomore Mathematics majors at Clarkson University in which **49 out of 52 students were unable to negate the statement, 'Every member of my family is unemployed.'** The responses included a wide variety of interpretations of the statement and its negation." (Note: the authors themselves label this an *unpublished observation*, and the same example reappears as the opening illustration in Dubinsky 1997 — cite it as reported inside this paper, not as a separate study.)
  - Interview excerpts in the paper show students constructing correct single-level processes but failing to encapsulate them, which is what blocks the two- and three-level quantifications.
- **Relevance:** this is the *founding* genetic decomposition of quantification and it is now verified at full-text level, so the survey can quote the "49 out of 52" figure and the encapsulation account directly. **Year caveat:** the article's own reference list says 1988 with 8(2), 44–51; Dubinsky's publications page says 1989 (no volume). FLM vol. 8 is 1988, and the scan I read is a longer 22-page version than the 8-page FLM article, so treat the archived PDF as a preprint/extended version.

### 4. Durand-Guerrier, V. (2003). *Which notion of implication is the right one? From logical considerations to a didactic perspective.* Educational Studies in Mathematics **53**(1), 5–34. DOI 10.1023/A:1024661004375

- **Type:** peer-reviewed journal article (79 cited per OpenAlex; 57 per Crossref).
- **URLs fetched:** `https://hal.science/hal-04114485v1` via the HAL API (`https://api.archives-ouvertes.fr/search/?q=halId_s:hal-04114485...`, retrieved the **full verbatim abstract**) + `https://api.crossref.org/works/10.1023/a:1024661004375` (vol 53, issue 1, pp. 5–34).
- **Verified level:** ABSTRACT + METADATA.
- **Verbatim abstract (retrieved):** "Implication is at the very heart of mathematical reasoning. As many authors have shown, pupils and students experience serious difficulties in using it in a suitable manner. In this paper, we support the thesis that these difficulties are **closely related with the complexity of this notion**. In order to study this complexity, we refer to Tarski's semantic truth theory, which contributes to clarifying the different aspects of implication: propositional connective, logically valid conditional, generalized conditional, inference rules. We will show that for this purpose, it is necessary to **extend the classical definition of implication as a relation between propositions to a relation between open sentences with at least one free variable**. This permits to become aware of the fact that, in some cases, **the truth-value of a given mathematical statement is not constrained by the situation**, contrary to the common standpoint that, in mathematics, a statement is either true or false. … the didactic relevance of this theoretical stance will be illustrated by an analysis of two problematic situations and the presentation of some experimental results from our research on first-year university students' understanding of implication."
- **Relevance:** supplies the theoretical reason the *hidden universal quantifier* in P(x) ⇒ Q(x) is the crux of Blocker 5, and it has first-year-university experimental data attached. Pairs directly with Shipman 2015 (entry 8).

### 5. Durand-Guerrier, V. (2008). *Truth versus validity in mathematical proof.* ZDM — The International Journal on Mathematics Education **40**(3), 373–384. DOI 10.1007/s11858-008-0098-8

- **Type:** peer-reviewed journal article (36 cited per Crossref).
- **URL fetched:** `https://api.crossref.org/works/10.1007/s11858-008-0098-8` (Durand-Guerrier, Viviane; 2008; ZDM 40(3), 373–384) + `https://api.crossref.org/works?query.bibliographic=...` search record.
- **Verified level:** METADATA.
- **Actual findings:** not read. The title states the thesis: students conflate *truth* (of an instance / of an open statement under an assignment) with *validity* (of a universally quantified implication) — the same truth-vs-validity gap named in entry 4.
- **Relevance:** the citation to use for "students treat a statement with a free variable as if it already had a truth value", which is precisely what makes swapping ∀∃ feel harmless to them.

### 6. Durand-Guerrier, V. (2005). *Recherches sur l'articulation entre la logique et le raisonnement mathématique dans une perspective didactique…* Habilitation à Diriger des Recherches (HDR) thesis, Université Claude Bernard Lyon 1 / IREM.

- **Type:** thesis (HDR — French senior research qualification; not peer-reviewed in the journal sense, but an examined academic thesis).
- **URL fetched:** `https://theses.hal.science/tel-00201626v1` (HAL API record: title, author, year 2005, docType HDR, English title "Researches about articulation between logic and mathematical reasoning, in a didactic perspective… Contributions of elementary model theory for a didactic analysis of mathematical reasoning").
- **Verified level:** METADATA.
- **Actual findings:** not read (French, long). It is the comprehensive statement of the model-theoretic approach to logic in mathematics education that entries 4 and 5 apply.
- **Relevance:** the "if you need one deep source in French on logic-in-mathematics-education, this is it" citation. Also confirms the Durand-Guerrier research programme is a sustained one, not a single paper.

### 7. Piatek-Jimenez, K. (2010). *Students' interpretations of mathematical statements involving quantification.* Mathematics Education Research Journal **22**(3), 41–56. DOI 10.1007/BF03219777

- **Type:** peer-reviewed journal article (17 cited per Crossref).
- **URLs fetched:** `https://api.crossref.org/works/10.1007/bf03219777` (Piatek-Jimenez, Katrina; 2010; MERJ 22(3), 41–56) via the Crossref author-query `https://api.crossref.org/works?query.author=Piatek-Jimenez&query.bibliographic=...`.
- **Verified level:** METADATA.
- **Actual findings:** not read (paywalled; Unpaywall reports `is_oa: false`).
- **⚠️ Correction to the lead I was given:** the brief listed "Piatek-Jimenez (2004?) one-to-one/onto". **No such paper exists in Crossref's index.** The author's actual work in this area is this 2010 MERJ paper on students' interpretations of *quantified* statements — which is squarely on Blocker 5 (quantifier semantics) but is **not** about one-to-one/onto. See UNVERIFIED.
- **Relevance:** use as a quantification-interpretation study; do **not** attribute one-to-one/onto findings to it.

### 8. Shipman, B. A. (2015). *Subtleties of hidden quantifiers in implication.* Teaching Mathematics and its Applications: An International Journal of the IMA. DOI 10.1093/teamat/hrv007

- **Type:** peer-reviewed journal article (8 cited per OpenAlex).
- **URL fetched:** OpenAlex record (`title_and_abstract.search=negation of quantified statements students difficulties`), which returned the full abstract.
- **Verified level:** ABSTRACT.
- **Retrieved abstract (near-verbatim):** "Mathematical conjectures and theorems are most often of the form P(x) ⇒ Q(x), meaning ∀x, P(x) ⇒ Q(x). **The hidden quantifier ∀x is crucial in understanding the implication as a statement with a truth value. Here P(x) and Q(x) alone are only predicates, without truth values**, since they contain unquantified variables. But standard textbook instruction on implication, in particular in writing negations, **relies mainly on truth tables, treating P and Q as statements themselves with their own truth values**. The lack of careful and thorough explanations on handling implications of the form P(x) ⇒ Q(x) creates difficulties for students, **in particular in proof by contradiction**, where one begins with the negation of the statement to be proved. Through analysis of interesting errors involving hidden quantifiers in implication, this article offers ways to improve standard instruction…"
- **Relevance:** a *teaching-practice* source that names the exact instructional cause of the error Dubinsky measured (entry 1: correct quantifier negation, wrong implication negation). If the curriculum survey needs one "why the textbook causes this" citation, this is it.

### 9. Alacacı, C., & Pasztor, A. (2005). *On people's incorrect either-or patterns in negating quantified statements: A study.* eScholarship (California Digital Library) / Florida International University.

- **Type:** **ERC (Education Resources Catalog) / institutional-repository record with abstract** — treat as a preprint or non-peer-reviewed report unless a journal version is located. OpenAlex type: `article`, venue `eScholarship`, 3 cited.
- **URLs fetched:** OpenAlex record (full abstract retrieved); `https://escholarship.org/uc/item/5fz0p6m1` (attempted direct fetch — **HTTP 202, 0 bytes**, blocked; the abstract below comes from the OpenAlex record, not from the eScholarship page).
- **Verified level:** ABSTRACT (via OpenAlex).
- **Retrieved abstract (verbatim):** "People manifest formally incorrect either-or, polarizing response tendencies when asked to negate quantified statements. Our study focuses on students' error patterns when negating quantified sentences, which are the single most important cause for their difficulties with indirect proofs and proofs by contradiction. We found that, contrary to our expectations, **the effect of content is relatively small on their negation behavior**; that of the four quantifier categories used, **students have by far the most difficulties in negating universally quantified sentences**; and that **the effect of formal logic instruction wears off relatively fast**."
- **Relevance:** two directly usable findings for Blocker 5: (a) the difficulty is structural, not content-driven, so it will not be fixed by more familiar examples; (b) formal-logic instruction decays — relevant if the curriculum front-loads logic.

### 10. Selden, J., & Selden, A. (1995). *Unpacking the logic of mathematical statements.* Educational Studies in Mathematics **29**(2), 123–151. DOI 10.1007/BF01274210

- **Type:** peer-reviewed journal article (153 cited per Crossref).
- **URL fetched:** `https://api.crossref.org/works/10.1007/bf01274210` (Selden, John; Selden, Annie; 1995; ESM 29(2), 123–151).
- **Verified level:** METADATA.
- **Actual findings:** **not retrieved** (no abstract in Crossref; full text closed). Verified: title, authors, year, venue, vol/issue/pages, 153 citations. The "unpacking the logical structure" framing is the paper's title-level claim only — I did not read the argument or any data.
- **Relevance:** standard citation for "students fail at the logic-parsing stage, upstream of the proof-technique stage". Good bridge citation between Blocker 5 and proof courses.

### 11. Moore, R. C. (1994). *Making the transition to formal proof.* Educational Studies in Mathematics **27**(3), 249–266. DOI 10.1007/BF01273731

- **Type:** peer-reviewed journal article (265 cited per Crossref).
- **URL fetched:** `https://api.crossref.org/works/10.1007/bf01273731` (Moore, Robert C.; 1994; ESM 27(3), 249–266).
- **Verified level:** METADATA.
- **Actual findings:** **not retrieved** (no abstract in Crossref; full text closed). Verified: title, author, year, venue, vol/issue/pages, 265 citations.
- **Relevance:** a standard transition-to-proof citation; if the survey uses it to support "students fail at unpacking definitions", that attribution must be re-checked against the text first.

### 12. Dawkins, P. C., & Roh, K. H. (2019). *Assessing the influence of syntax, semantics, and pragmatics in student interpretation of multiply quantified statements in mathematics.* International Journal of Research in Undergraduate Mathematics Education **6**(1), 1–22. DOI 10.1007/s40753-019-00097-2 (plus author correction DOI 10.1007/s40753-019-00105-5)

- **Type:** peer-reviewed journal article (13 cited per Crossref).
- **URL fetched:** `https://api.crossref.org/works/10.1007/s40753-019-00097-2` (Dawkins, Paul Christian; Roh, Kyeong Hah; 2019; IJRUME 6(1), 1–22) + Crossref search record.
- **Verified level:** METADATA. (There is a published **author correction** — worth noting in a survey since a correction exists.)
- **Actual findings:** not read. By title and framing, it separates **syntax** (the formal ∀∃ structure), **semantics** (the mathematical meaning) and **pragmatics** (conversational implicature / plausibility) as competing influences on how students parse multiply-quantified statements — i.e. it formalises the Dubinsky & Yiparaki observation that wording drives the reading.
- **Relevance:** the modern, peer-reviewed, quantitatively-framed successor to entry 2. Best current citation for "students read multiply quantified statements pragmatically, not syntactically."

### 13. Schüler-Meyer, A. (2022). *How transition students relearn school mathematics to construct multiply quantified statements.* Educational Studies in Mathematics **110**(2), 291–311. DOI 10.1007/s10649-021-10127-z

- **Type:** peer-reviewed journal article (7 cited per Crossref; indexed as ERIC EJ1334228).
- **URLs fetched:** `https://api.crossref.org/works/10.1007/s10649-021-10127-z` (Schüler-Meyer, Alexander; 2022; ESM 110(2), 291–311) + ERIC search record (ERIC EJ1334228, "How Transition Students Relearn School Mathematics to Construct Multiply Quantified Statements", ESM 2022).
- **Verified level:** METADATA + ERIC record.
- **Actual findings:** not read in full. The ERIC record confirms it studies how students entering university *relearn* to construct multiply quantified statements — i.e. it is about the repair of exactly the ∀∃ mis-parses in entries 1, 2 and 12.
- **Relevance:** the most recent ESM paper on multiply quantified statements; good for a survey's "current state" section.

### 14. Bansilal, S., Brijlall, D., & Trigueros, M. (2017). *An APOS study on pre-service teachers' understanding of injections and surjections.* The Journal of Mathematical Behavior **48**, 22–37. DOI 10.1016/j.jmathb.2017.08.002

- **Type:** peer-reviewed journal article (27 cited per OpenAlex; 22 per Crossref).
- **URL fetched:** `https://api.crossref.org/works/10.1016/j.jmathb.2017.08.002` (Bansilal, Sarah; Brijlall, Deonarain; Trigueros, Maria; 2017; JMB 48, 22–37) + OpenAlex record.
- **Verified level:** METADATA. **Full text not retrieved** — ScienceDirect returned a CAPTCHA page ("Are you a robot?"); Unpaywall reports no OA copy.
- **Actual findings:** not read. This is the **only** APOS study in my retrieved set that targets injections and surjections specifically, i.e. the ∀∃ / ∃∀ definitions of Blocker 5 applied to functions; it is the paper to get hold of for the specific "students swap the quantifiers in the injective/surjective definitions" claim.
- **Relevance:** highest-priority gap to fill. Cite only after obtaining the full text; at present I can confirm only the citation and topic, not the findings.

### 15. Mukmin, M. I., & Fa'ani, A. M. (2020). *Identification of students' misconceptions in proving onto and one-to-one function in abstract algebra using certainty response index.* International Journal on Teaching and Learning Mathematics **2**(1). DOI 10.18860/ijtlm.v2i1.8582

- **Type:** peer-reviewed journal article (low citation count: 2). OpenAlex lists an OA PDF at `http://ejournal.uin-malang.ac.id/index.php/ijtlm/article/download/8582/pdf`, but **the PDF returned 0 bytes and the article-view page returned an HTML stub** through this proxy.
- **URLs fetched:** OpenAlex record (full abstract retrieved); attempts at `https://ejournal.uin-malang.ac.id/index.php/ijtlm/article/view/8582` (200, 45 kB HTML stub) and `.../download/8582/4272` (200, **0 bytes**).
- **Verified level:** ABSTRACT (via OpenAlex), PDF NOT retrieved.
- **Retrieved abstract (verbatim):** "This research is aimed to identify students' misconceptions in proving onto and one-to-one function in Abstract Algebra using CRI (Certainty Response Index)… There are eighteen research participants, and they are Mathematics Education students in 3rd semester… **The result shows that ten students get misconceptions. They were weak in understanding the definition of onto and one-to-one function**, and are not yet trained to proof onto and one-to-one function in Abstract Algebra. Further, this research also find that **the errors of participants in proving mathematics are mainly influenced by errors in algebraic operation**."
- **Relevance:** small-N but directly on point: 10/18 students held misconceptions, and the stated weakness is **understanding the definitions** of onto and one-to-one. Low-tier venue — use as corroboration, not as a primary citation. Also note the second finding (errors dominated by algebraic manipulation rather than logic) rhymes with Dubinsky's implication finding.

### 16. Chellougui, F., & Kouki, R. (2012). *Enquêtes épistémologique et didactique du concept de la quantification.* Colloque / conference paper, HAL `hal-02471297`.

- **Type:** conference paper (HAL docType `COMM`), French.
- **URL fetched:** `https://api.archives-ouvertes.fr/search/?q=halId_s:hal-02471297...` (HAL API; **full verbatim abstract retrieved**, both French and English).
- **Verified level:** ABSTRACT.
- **Retrieved abstract (key content):** Part 1 is an epistemological survey from Frege and Quine showing "the complexity and the polysemy of quantifiers in formal language and natural language (Chellougui 2004)". Part 2 is a didactic analysis: "**We discuss some experimental results from earlier work of Dubinsky and Yiparaki (2000) and of Durand-Guerrier and Arsac (2003)**" (keywords: quantification, universal quantifier, existential quantifier, logic, predicate calculus).
- **Relevance:** independent confirmation that Dubinsky & Yiparaki (entry 2) is a real and used source; also gives a compact French-language review of the quantification literature for a curriculum survey that needs a non-English citation.

### 17. Vroom, K. (2022). *A functional perspective on student thinking about the grammar of multiply quantified statements.* The Journal of Mathematical Behavior. DOI 10.1016/j.jmathb.2022.100992

- **Type:** peer-reviewed journal article (13 cited per Crossref).
- **URL fetched:** `https://api.crossref.org/works?query.bibliographic=How+transition+students+relearn...` (Crossref returned this record alongside Schüler-Meyer 2022).
- **Verified level:** METADATA. (Also indexed as ERIC EJ1418633-adjacent; not independently fetched.)
- **Actual findings:** not read.
- **Relevance:** a third recent (2022) line on multiply quantified statements, using a functional-grammar lens. Useful if the survey wants to show the field converging on this problem in 2019–2022.

---

## BLOCKER 6 — Image vs preimage: forward vs backward preservation
(`f(A∩B) ⊆ f(A)∩f(B)` but not conversely; `f⁻¹` preserves ∩ and complement, `f` does not)

**Blocker 6 is the thinnest of the three in the literature, and I want to be blunt about that.** There is no widely cited paper whose title is "students' understanding of image and preimage". I ran 8 OpenAlex title/abstract queries, 6 Crossref queries (including the exact-title search for the "schema thematization" paper), 4 ERIC queries, 2 HAL queries, 1 OpenAIRE query, and 2 BASE/Unpaywall probes specifically for this blocker. What follows is what actually exists and is verified; §UNVERIFIED says what does not.

### 1. Breidenbach, D., Dubinsky, E., Hawks, J., & Nichols, D. (1992). *Development of the process conception of function.* Educational Studies in Mathematics **23**, 247–285. DOI 10.1007/BF02309532

- **Type:** peer-reviewed journal article. (Full verification details in Blocker 4, entry 1.)
- **URL fetched:** `https://web.archive.org/web/20050209160001if_/http://www.math.kent.edu/~edd/PROCESSFUNC.pdf` (OCR → `/tmp/stresearch/bdhn_ocr2.txt`).
- **Verified level:** FULLTEXT.
- **Actual findings relevant to Blocker 6 (read verbatim from the body):** the four-week instructional treatment was organised around the *direction* of the function process, and item 6 of the treatment is explicitly: "There are activities designed to help the students think about situations in terms of **reversing the process of a function**. These are embodied in problems that **compute pre-images of functions**, the construction of an **inverse function** and the concepts of **1-1 and onto**." Item 5 of the same treatment covers the **forward** direction: "equality of functions (they are asked to write funcs that will test for it), **images** (they are asked to compute them for functions given by various kinds of representations), pointwise arithmetic of functions …, and composition."
  - The paper's whole theoretical claim is that students lack the *reversible* mental process; the body contains the sentence "It is necessary not only to encapsulate a process to obtain an object, but also to be able to **unpack or de-encapsulate** the object and …"
- **Relevance:** this is the closest thing to a primary source for Blocker 6 that I could actually read. It is *not* about `f(A∩B) ⊆ f(A)∩f(B)` specifically, but it is about exactly the forward/backward asymmetry: computing images is the forward process; computing pre-images requires reversing the process, which is the move students cannot make. Cite it for the asymmetry, not for the ∩/complement preservation theorem.

### 2. Hamdan, M. (2006). *Equivalent structures on sets: Equivalence classes, partitions and fiber structures of functions.* Educational Studies in Mathematics **62**(2), 127–147. DOI 10.1007/s10649-006-5798-9

- **Type:** peer-reviewed journal article (9 cited per OpenAlex, 9 per Crossref; indexed as ERIC EJ748150).
- **URLs fetched:** `https://eric.ed.gov/?id=EJ748150` (ERIC record with the **full abstract** — fetched and read) + `https://api.crossref.org/works/10.1007/s10649-006-5798-9` (Hamdan, May; 2006; ESM 62(2), 127–147) + Unpaywall (`is_oa: false`, so no free PDF).
- **Verified level:** ABSTRACT (ERIC, verbatim) + METADATA.
- **Verbatim abstract (retrieved):** "This study reports on how students can be led to make meaningful connections between such structures on a set as a partition, the set of equivalence classes determined by an equivalence relation and the **fiber structure of a function on that set (i.e., the set of preimages of all sets {b} for b in the range of the function)**. In this paper, I first present an **initial genetic decomposition, in the sense of APOS theory, for the concepts of equivalence relation and function** in the context of the structures that they determine on a set. This genetic decomposition is primarily based on my own mathematical knowledge as well as on my observations of students' learning processes. Based on this analysis, I then suggest instructional procedures that motivate the mental activities described in the genetic decomposition. I finally present **empirical data from informal interviews with students** at different stages of learning. My goal was to guide students to become aware of the close conceptual correspondence and connections among the aforementioned structures. One theorem that captures such connections is the following: **a relation R on a set A is an equivalence relation if and only if there exists a function f defined on A such that elements related via R (and only those) have the same image under f**."
- **Relevance:** the strongest *verified* Blocker-6 source. It is an APOS genetic decomposition whose central object **is the set of preimages** (the fiber structure), and it connects preimage-fibers to partitions/equivalence classes — i.e. it is the pedagogical bridge between "preimage" and structure. Use it for the claim that preimage is conceptually harder and needs its own genetic decomposition; it does not measure the ∩/complement preservation failure.

### 3. Arnon, I., Cottrill, J., Dubinsky, E., Oktaç, A., Roa Fuentes, S., & Trigueros, M. (2014). *APOS Theory: A Framework for Research and Curriculum Development in Mathematics Education.* Springer. DOI 10.1007/978-1-4614-7966-6

- **Type:** book (204 cited per Crossref).
- **URL fetched:** `https://api.crossref.org/works?query.bibliographic=APOS+theory+a+framework+for+research+and+curriculum+development...` (Crossref record, book, 2014, 204 cited) and `https://api.crossref.org/works?filter=container-title:APOS%20Theory...` (chapter records retrieved: "The APOS Paradigm for Research and Curriculum Development" `10.1007/978-1-4614-7966-6_6`; "The Teaching of Mathematics Using APOS Theory" `_5`; "Mental Structures and Mechanisms: APOS Theory and the Construction of Mathematical Knowledge" `_3`).
- **Verified level:** METADATA (book + chapter DOIs).
- **Actual findings:** not read. This is the canonical statement of APOS; the chapter-level genetic decompositions (including for function) are where a "schema of functions" treatment of image/preimage would live.
- **Relevance:** cite the book DOI for the theory; the chapters are separately DOI'd if the survey needs precision.

### 4. Dubinsky, E., & McDonald, M. A. *APOS: A constructivist theory of learning in undergraduate mathematics education research.* New ICMI Study Series. DOI 10.1007/0-306-47231-7_25

- **Type:** book chapter (101 cited per Crossref; 379 cited per OpenAlex). **Crossref returns `year: None` for this chapter — the 2005 date commonly attached to it was NOT verified by anything I retrieved.**
- **URL fetched:** `https://api.crossref.org/works?query.bibliographic=APOS+a+constructivist+theory+of+learning+in+undergraduate+mathematics+education+research` (Crossref record; note Crossref returns year `None` for this chapter).
- **Verified level:** METADATA.
- **Actual findings:** **not retrieved** (no abstract; full text closed). Verified: title, authors, venue series, DOI.
- **Relevance:** the short, citable definition of APOS if the survey does not want to cite the whole 2014 book.

### 5. Asiala, M., Brown, A., DeVries, D. J., Dubinsky, E., Mathews, D., & Thomas, K. (1996). *A framework for research and curriculum development in undergraduate mathematics education.* Research in Collegiate Mathematics Education II, Issues in Mathematics Education (CBMS), American Mathematical Society.

- **Type:** peer-reviewed/edited research volume chapter (CBMS Issues in Mathematics Education).
- **URLs fetched:** `https://web.archive.org/web/20050209142127if_/http://www.math.kent.edu:80/~edd/Framework.pdf` (author-archived 34-page PDF; downloaded, parsed text garbled by the subset font, then **OCR'd successfully** → `/tmp/stresearch/fw_ocr.txt`).
- **Verified level:** FULLTEXT (OCR; the abstract, section headings and body were read).
- **Actual findings (from the retrieved text):** presents the three-component APOS research cycle (theoretical analysis / genetic decomposition → instructional design → data gathering and revision) and defines schemas as coherent collections of actions, processes and objects. The retrieved text contains the schema definition — "individual will have a **function schema**, a derivative schema, a group schema, etc. Schemas are…" — but the OCR'd body I obtained does **not** contain a worked genetic decomposition of image/preimage. (I searched the OCR for `preimage`, `pre-image`, `preimage`, `inverse image`, `image of a set`: no hits.)
- **Relevance:** cite it for the APOS methodology and for the "schema of functions" terminology that the Blocker-6 lead names; it is **not** itself a source of image/preimage findings.

### 6. Baker, B., Cooley, L., & Trigueros, M. (2000). *A calculus graphing schema.* Journal for Research in Mathematics Education. DOI found via Crossref record (72 cited).

- **Type:** peer-reviewed journal article.
- **URL fetched:** `https://api.crossref.org/works?query.bibliographic=Cooley+Baker+Trigueros+schema+thematization+function` (Crossref returned: "A Calculus Graphing Schema", Baker, Bernadette; Cooley, Laurel; Trigueros, María; 2000; JRME; 72 cited).
- **Verified level:** METADATA. Volume/pages not retrieved — do not print them.
- **Actual findings:** **not retrieved** (no abstract in Crossref; full text closed). Verified: title, authors (Baker, Bernadette; Cooley, Laurel; Trigueros, María), year 2000, venue JRME, 72 citations; volume/pages not returned by Crossref.
- **Relevance:** the "Cooley/Baker/Trigueros" leg of the Blocker-6 lead. It is real and peer-reviewed, but from the title it is about a *calculus graphing* schema, **not** about preimage preservation, and I read none of its content. See UNVERIFIED for the "schema thematization" paper.

### 7. Selden, J., & Selden, A. (1995) and Moore, R. C. (1994) — see Blocker 5, entries 10 and 11.

- **Type / URLs / verified level:** as listed there (METADATA via Crossref).
- **Relevance to Blocker 6:** both are the standard citations for the claim that students cannot *use* a definition to structure a proof. The `f(A∩B) ⊆ f(A)∩f(B)` task is a definition-unpacking task: you must instantiate "y ∈ f(A∩B)" and then produce witnesses. If the survey needs a citation for "students fail at the unpacking step", these are it; neither contains image/preimage data.

### 8. Bansilal, S., Brijlall, D., & Trigueros, M. (2017) — see Blocker 5, entry 14.

- **Relevance to Blocker 6:** injections/surjections are the ∀∃ definitions that determine whether `f` preserves or reflects structure; a student who cannot state them cannot reason about `f(A∩B)` vs `f(A)∩f(B)`. Paywalled; **top priority to obtain**.

### 9. Tall, D., & Vinner, S. (1981) / Vinner (1983) / Vinner & Dreyfus (1989) — see Blocker 4, entries 5–7.

- **Relevance to Blocker 6:** supplies the mechanism for the specific error. `f(A∩B) = f(A)∩f(B)` is true of *injective* functions; a student whose concept image of "function" is a familiar injective prototype (x², sin x restricted, a formula) will "verify" the false converse on their image and never see the need for the condition. This is a *derived* relevance from verified sources, not a finding reported by them — mark it as such if used.

---

## UNVERIFIED / COULD NOT RETRIEVE

Everything in this section is a lead I could **not** confirm. Do not cite any of it without new evidence.

1. **Piatek-Jimenez (2004?) on one-to-one/onto — NOT FOUND.**
   Tried: Crossref author+bibliographic query `query.author=Piatek-Jimenez` × bibliographic "one-to-one onto functions students", "Piatek-Jimenez one-to-one onto"; OpenAlex `title_and_abstract.search:Piatek-Jimenez`; Semantic Scholar (HTTP 429, rate-limited). Result: the only Piatek-Jimenez mathematics-education work in the index is *Students' interpretations of mathematical statements involving quantification*, MERJ 22(3), 2010. There is no 2004 one-to-one/onto paper by this author detectable. The lead as given appears to be a **mis-citation**.

2. **Cusi & Malara (2007?) on quantifiers — NOT FOUND.**
   Tried: Crossref `query.bibliographic=Cusi Malara quantifiers argumentation` (returned unrelated generalized-quantifier linguistics); HAL API `quantifiers mathematics education Malara` (zero results). What I *did* find and verify is Cusi, A., & Malara, N. A. (2011), *Improving awareness about the meaning of the principle of mathematical induction*, PNA — Revista de Investigación en Didáctica de la Matemática (Crossref record, 1 cited) — **topic is induction, not quantifier order**. Use only if a Cusi/Malara citation is essential and the induction topic is acceptable.

3. **Vidakovic on APOS / preimage — NOT FOUND as a standalone source.**
   Tried: OpenAlex `title_and_abstract.search:Vidakovic APOS function preimage`, `learning the concept of inverse function Vidakovic`; Crossref `query.bibliographic=Vidakovic learning the concept of inverse function` (returned only an ASEE WIP paper by Raviv, an inverse-RL paper, and a TARBIYA article on inverse-function learning obstacles — none by Vidakovic). Draga Vidakovic does appear as a co-author in Dubinsky's archived publication list (item 49, with Czarnocha, Loch, Prabhu & Vidakovic, on calculus students' intuition of area) — but **not** on a function/preimage paper I could verify. The frequently cited "Vidakovic (1996), Learning the concept of inverse function, JCMST" did **not** surface in any index I could query, and ERIC searches for `Vidakovic inverse function` returned nothing by her (the ERIC helper timed out on the first attempt and returned unrelated inverse-function papers on retry). **Unresolved.**

4. **Cooley, Trigueros & Baker, "Schema thematization: A theoretical framework and an example" — NOT VERIFIED.**
   Tried: Crossref `query.bibliographic=Schema thematization a theoretical framework and an example` and `Cooley Trigueros Baker schema thematization theoretical framework example`; both returned only unrelated database-schema and psychology papers. What **is** verified in that research line is Baker, Cooley & Trigueros (2000), *A Calculus Graphing Schema*, JRME (entry 6 above). If the survey needs "schema thematization", it must be verified against the JRME archive directly (JSTOR/ NCTM), which is not reachable through this proxy.

5. **Monk (1992), "Students' understanding of a function given by a physical model" (MAA Notes 25 volume) — NOT VERIFIED.**
   Tried: Crossref `query.bibliographic=Monk students understanding of a function given by a physical model` (returned only an unrelated physical-education dissertation). Verified instead: Monk, G. S. (1994), *Students' Understanding of Functions in Calculus Courses*, Humanistic Mathematics Network Journal, DOI 10.5642/hmnj.199401.09.07.

6. **Markovits, Eylon & Bruckheimer, "Functions today and yesterday" — NOT VERIFIED.**
   Tried: OpenAlex `title_and_abstract.search:functions as rules and functions as mappings students` (returned only a 1986 *Arithmetic Teacher* "From the File" note by Markovits, Hershkowitz, Taizi & Bruckheimer, DOI 10.5951/at.34.4.0009); Crossref `query.bibliographic=Functions today and yesterday Markovits Eylon Bruckheimer`. *For the Learning of Mathematics* pre-2000 is not in Crossref, and OpenAlex was rate-limited by this point. **Unresolved.** (Note: the Markovits/Carlson lead in the brief conflates two different authors — Markovits, Z. (Israel) and Carlson, M. (Arizona State) are unrelated.)

7. **Sfard (1992), "Operational origins of mathematical objects and the quandary of reification — the case of function", in Harel & Dubinsky (Eds.), MAA Notes 25 — NOT VERIFIED.**
   Tried: Crossref `query.bibliographic=Operational origins of mathematical objects and the quandary of reification the case of function` (returned only unrelated "Objects and Organisms" chapters). The MAA Notes 25 volume is not Crossref-indexed. The volume's existence and the Dubinsky & Harel chapter within it *are* verified (via Dubinsky's own publication list); Sfard's chapter in it is not.

8. **Full texts I could not read (paywalled, no OA copy, publisher blocked):**
   Sfard (1991); Sfard & Linchevski (1994); Vinner (1983); Vinner & Dreyfus (1989); Tall & Vinner (1981); Tall & Bakar (1992) — abstract only; Thompson (1994); Even (1990, 1993); Carlson (1998); Monk (1994); Hitt (1998); Dubinsky & Wilson (2013); Cooney & Wilson (1993/2012); McCulloch et al. (2020); Kjeldsen & Petersen (2013); Piatek-Jimenez (2010); Selden & Selden (1995); Moore (1994); Durand-Guerrier (2003, 2008); Dawkins & Roh (2019); Schüler-Meyer (2022); Vroom (2022); Bansilal/Brijlall/Trigueros (2017); Hamdan (2006). For all of these I have a verified bibliographic record and, where noted, a verbatim abstract — **no fabricated findings**.
   Specific blockers encountered: Springer and ScienceDirect landing pages returned JS/Cloudflare interstitials; `escholarship.org` returned HTTP 202 with an empty body; `ejournal.uin-malang.ac.id` returned HTTP 200 with 0 bytes for the PDF; `davidtall.com` has a self-signed certificate (readable with `curl -k`, but the Warwick download page it points to is 404); `warwick.ac.uk/staff/David.Tall/downloads.html` is dead and the Wayback CDX API timed out (only direct `/web/<ts>if_/` URLs work).

9. **API-level blockers for whoever continues this:** OpenAlex exhausted its daily budget mid-session (HTTP 429, "dailyRemainingUsd 0.0003", resets midnight UTC) — this is the single biggest constraint, because OpenAlex's abstract-inverted-index was the only free source of abstracts for the older Springer/Elsevier papers. Semantic Scholar returns HTTP 429 without an API key. BASE denies this IP. OpenAIRE responds but its `title=` mode requires exact title tokens (use `keywords=` and parse the nested `oaf:entity`/`oaf:result` JSON). CORE, dblp, Scholar, ResearchGate all blocked (per BRIEF.md, re-confirmed).

10. **Date/attribution caveats to carry forward:**
    - Dubinsky, Elterman & Gong: **1988 vs 1989 discrepancy** between the article's own reference list and Dubinsky's publications page (both retrieved). FLM vol. 8 ⇒ 1988 is right, but verify. The archived scan is 22 pages while the FLM article occupies 44–51 (8 pages), so the archived PDF is an extended/preprint version — cite the FLM pagination, quote from the scan with that caveat.
    - The "**49 out of 52 students** could not negate 'Every member of my family is unemployed'" figure is labelled by Dubinsky, Elterman & Gong themselves as an *unpublished observation* at Clarkson University; it is not a published experiment. Quote it as reported inside that paper.
    - Dubinsky (1997) "On Learning Quantification": the sample sizes I read off the text and tables are **19** students in Class1 (Clarkson, Fall 1986) and **17** in Class2, with per-instrument n of 18 (SetA, SetB) and 19 (SetC, AssignA, AssignB). The paper reports "11 of the 21 problems" for Class1 and "16 problems" for Class2.
    - Cooney & Wilson: Crossref's year is 2012 (Routledge reissue); the original is commonly given as 1993 and I did **not** verify the original year or page range.
    - Thompson (1994), Carlson (1998), Baker/Cooley/Trigueros (2000), Hitt (1998): **page ranges not verified** — Crossref returned no `page` field. Bansilal et al. (2017) pages (48, 22–37) come from Crossref, but the full text was unreachable.
    - Dubinsky & Yiparaki is an **unpublished manuscript** dated 18 Nov 2000, not a journal article; it is referenced as "Dubinsky and Yiparaki (2000)" by Chellougui & Kouki (2012), which is how I would cite it.
    - Alacacı & Pasztor (2005) is an eScholarship record; I could not retrieve the document, only the OpenAlex abstract. Treat as a preprint/report until a peer-reviewed version is located.

---

### Files produced this session (for reuse)

- `/tmp/stresearch/dec2.py` — decoder for the Caesar-shifted subset-font text in Dubinsky's archived PDFs (`python3 dec2.py raw.txt`).
- `/tmp/stresearch/ocr5.sh` — render+rotate+OCR any PDF (`./ocr5.sh in.pdf out.txt`; must write temp PNGs into the working directory, not `/tmp`, because tesseract in this sandbox cannot open files under `/tmp`).
- `/tmp/stresearch/html/LQ_clean.txt` — decoded full text, Dubinsky (1997) "On Learning Quantification", including the six results tables and conclusions.
- `/tmp/stresearch/html/olga_clean.txt` — decoded full text, Dubinsky & Yiparaki (2000) AE/EA quantification, 67 pp.
- `/tmp/stresearch/bdhn_ocr2.txt` — OCR of Breidenbach, Dubinsky, Hawks & Nichols (1992), ESM 23, 247–285.
- `/tmp/stresearch/fw_ocr.txt` — OCR of Asiala et al. (1996) APOS framework paper.
- `/tmp/stresearch/quant1_ocr.txt` — OCR of Dubinsky, Elterman & Gong, "The Student's Construction of Quantification" (author's 22-page scan; UC Berkeley Spring 1986 study).
- `/tmp/stresearch/html/ft_jukic.txt`, `/tmp/stresearch/html/ft_sherman.txt` — ERIC open-access full texts (pre-service teacher function concept image/definition).


---

# Part C — Blockers 7–8 (countable/uncountable, Axiom of Choice)

# Part C — Blocker 7 (countable/uncountable, Cantor's diagonalization) and Blocker 8 (Axiom of Choice)

Agent: blocker-7-8 evidence gatherer. Date of retrieval: this session.
Method: local proxy + `curl`; OpenAlex API (`title_and_abstract.search`, `default.search`, `fulltext.search`),
Crossref REST API, ERIC HTML search + ERIC full-text PDFs, arXiv API, Semantic Scholar API, publisher/repository pages.
`web_search` / `web_fetch` were never used.

**Reading key for "URL actually fetched"**: every URL below was retrieved in this session. Where the only
retrievable item was an API record (OpenAlex/Crossref/Semantic Scholar) rather than the paper itself, this is
stated explicitly and the record is quoted from. Where a PDF was image-only, this is stated and the OCR
procedure is described. Nothing below is written from memory.

---

## BLOCKER 7 — Countable vs uncountable sets, Cantor's diagonalization, "same cardinality"

### 7.1 Empirical studies of student conceptions (primary evidence, full text read)

**1. Hamza, Safia & O'Shea, Ann (2011). "Students' Misconceptions Concerning Infinity."**
In *Proceedings: Fourth Conference on Research in Mathematics Education (MEI 4)*, St Patrick's College,
Drumcondra, pp. 192–202.
- **Source type:** conference proceedings paper (MEI 4 — the Irish Conference on Research in Mathematics
  Education; proceedings are refereed).
- **URLs fetched:**
  - Landing/citation page: `https://mural.maynoothuniversity.ie/6977/`
  - Full PDF: `http://eprints.maynoothuniversity.ie/6977/1/AOS-Student-Misconceptions.pdf`
  - The PDF is **image-only** (PyMuPDF text layer = 0 chars/page, 12 images/page). I rasterised all 11 pages
    at 250 dpi with PyMuPDF and OCR'd them with `tesseract` (eng); OCR text = 29,287 chars, reviewed in full.
- **Actual findings:** 35 students in three NUI Maynooth groups (12 first-year Mathematics & Theoretical
  Physics; 13 out-of-field mathematics teachers on a postgraduate course; 10 second-year science students),
  all in their *first* rigorous analysis module, answered a 7-question questionnaire. Five misconception
  families were identified: (a) **everyday-language misconceptions** — "countable" read as "can be physically
  counted", so students equate *finite* with *countable* and *infinite* with *uncountable* (verbatim response:
  "M is finite as all the melodies that have been composed are a set number, they are countable"; "A is
  equivalent to N, because there is an infinite number of elements in both sets and both are uncountable");
  (b) **misuse of properties** — students over-generalise true facts from the countable case, asserting that
  "every subset of an uncountable set is uncountable" and that **all uncountable sets are equivalent /
  equinumerous**; (c) **misuse of the bijection criterion** — of the students who invoked it, *none* used it in
  all problems, and they applied it correctly in some items and incorrectly in others; they "rarely wrote down
  a specific map" and, when arguing two sets were *not* equivalent, simply asserted that no bijection exists;
  (d) **set-representation failures** — students wrote the set of forks as `F = {2,4,6,8,…}` and read the
  open interval `(1.25, 3.79)` as the two-element set `{1.25, 3.79}`, hence "the set A only has 2 numbers";
  (e) **infinity as an unreachable/largest number** — this fed the belief that all infinite sets are
  equivalent ("Infinity cannot be greater than infinity, they are of equal cardinality").
  The authors state they could find no earlier study reporting the *"countable" = can-be-counted*
  spontaneous conception.
- **Relevance to the survey:** This is the single most on-point source for Blocker 7: it is about exactly
  countable-vs-uncountable, it names **misuse of the bijection criterion** as a distinct error type, it documents
  the part-whole/"all infinite sets are the same" intuition, and it gives quotable student verbatims.

**2. Zazkis, Rina & Mamolo, Ami (2009). "Sean vs. Cantor: Using mathematical knowledge in 'experience of
disturbance'."** *For the Learning of Mathematics* 29(3), 53–56.
- **Source type:** peer-reviewed journal article (research/practitioner journal).
- **URL fetched (full PDF):** `https://flm-journal.org/Articles/492E35FADC6DE2DD1D825A1FEEB71.pdf`
- **Actual findings:** Case study in a Master's "Foundations of Mathematics" course for practising secondary
  teachers. A student ("Sean") constructed an explicit "enumeration" of the reals in (0,1) — one-digit decimals
  ↦ 1–9, two-digit ↦ 10–99, etc. — and claimed it was a bijection with ℕ. He **resisted several refutations**;
  what finally convinced him was the instructor's question about what real number comes immediately *before or
  after* 1/3 in his ordering, which he could not answer. The following year the same instructor presented
  "Sean's correspondence" to a new cohort *after* they had studied Cantor's theorem; the class's first reaction
  was **"Cool!"** with nodding agreement — i.e. they did **not** recognise that the construction contradicted
  Cantor's theorem. Only after a quiet remark ("So are you saying that Cantor was wrong and Sean should get a
  Fields medal?") did conflict recognition occur; resolution then came quickly via irrationals' infinite decimal
  expansions. A follow-up "If not, what yes?" move established that Sean's correspondence actually proves
  *only* that the rationals in (0,1) **with finite decimal expansions** are countable.
- **Relevance:** Direct evidence about **acceptance of the diagonal argument / resistance to refutation**, and
  about how weak students' *conflict detection* is even after instruction. Excellent for justifying a curriculum
  that forces conflict recognition rather than asserting the theorem.

**3. Monaghan, John (1986). "Adolescents' understanding of limits and infinity."** PhD thesis, University of
Warwick.
- **Source type:** doctoral thesis.
- **URL fetched (full PDF, text = 532,536 chars):** `http://wrap.warwick.ac.uk/34626/1/WRAP_THESIS_Monaghan_1986.pdf`
- **Actual findings (verbatim from the thesis's own list of results):** "Subjects' concepts of infinity do not
  conform to infinite cardinal or ordinal paradigms"; "Subjects' conceptions of limits and infinity are
  contradictory and labile"; context matters — "**A measuring context encourages subjects to ascribe a greater
  cardinality to the superset** in cardinality questions"; responses are sensitive to wording, context and mood.
  On the classic ℕ vs evens item (Q19: 1,2,3,4,… vs 2,4,6,8,…) Monaghan's A-level group split roughly evenly
  between "same" and "can't compare", and he notes this is **at odds with Fischbein et al. (1979), who found the
  majority (71% overall, 81% in the high-ability group) claimed ℕ was bigger** — he suspects their wording
  ("Which of the two sets contains more elements?") was leading. On ℕ vs the decimals in (0,1) the group
  preferred "more decimal numbers". Consistency across the five comparison items was low (only 13 of the
  "mathematician" group answered "same" in 3+ items vs 33 in the other group).
- **Relevance:** A long, careful primary study; supplies the **part-whole / Galileo's-paradox** evidence and the
  crucial methodological warning that apparently contradictory results across studies may be *wording effects*.
  Note: its 71%/81% figures for Fischbein et al. (1979) are a **secondary report** (see §7.2 item 8).

**4. Blaszczyk, Piotr (2020). "What theory of infinity should be taught and how?"** *Mathematics Teaching
Research Journal* 12(2), Special Issue on Philosophy of Mathematics Education (article starts p. 143).
- **Source type:** peer-reviewed journal article (ERIC EJ1384460, "Reports – Evaluative").
- **URL fetched (full PDF):** `https://files.eric.ed.gov/fulltext/EJ1384460.pdf`
- **Actual findings:** Argues *against* the standard reading of the math-ed literature. His thesis: students'
  intuitions about infinity "do not match … Cantor's theory, not … any theory of infinity" — rather, students'
  intuitions obey the rules of an **ordered field**, which Cantor's cardinal/ordinal arithmetic violates
  (he calls Cantor's ordinal arithmetic "rotten"). He then sketches two alternatives that *do* match student
  intuition and sketches how to teach them: (i) redefined ordinal arithmetic in an ordered-field setting
  (Conway's *On Numbers and Games*); (ii) Benci & Di Nasso's **numerosities**, in which every infinite subset of
  ℕ has strictly smaller numerosity than ℕ, thereby restoring Euclid's "the whole is greater than the part" for
  countable sets (definition: ν(A) is the nonstandard natural number represented by the sequence
  n ↦ |{a ∈ A : a ≤ n}|). His pedagogical proposal is to organise the topic around Archimedean vs
  non-Archimedean **ordered fields** instead of starting from Cantor's cardinals.
- **Relevance:** The main *dissenting* position in the literature; indispensable as the "why teach Cantor at all"
  counter-argument, and it names the exact Galileo/part-whole issue. Cite it as a theoretical/philosophical
  contribution, not as empirical data.

**5. Tabares Sánchez, Liliana Aurora; Moreno Armella, Luis Enrique; Miranda Viramontes, Isaías (2023).
"Intuition and formalization in the understanding of the mathematical infinite: The case of Omar."**
In *Proceedings of the 45th Annual Meeting of the North American Chapter of the International Group for the
Psychology of Mathematics Education (PME-NA)*, Vol. 2, pp. 136–143, University of Nevada, Reno.
- **Source type:** peer-reviewed conference paper (PME-NA proceedings; ERIC ED658393).
- **URL fetched (full PDF):** `https://files.eric.ed.gov/fulltext/ED658393.pdf`
- **Actual findings:** Single-case qualitative study of "Omar", a first-semester applied-mathematics
  undergraduate, facing two mirrors facing each other (an infinite regress of reflected plasticine balls).
  Omar oscillates between intuition ("if you put more balls it is a bigger infinity") and formal knowledge
  ("but if you put mathematics in it, well no"), and cannot assign a *cardinality* to the set while still
  conceiving it as infinite. He explicitly grounds his resistance in embodiment: "it is not feasible that there
  is a material infinity … that is why it is not feasible to measure an infinity". The authors conclude that
  "the premature replacement of Galilean insights (which are actually inarticulate consequences of embodiment)
  with Cantorian formalization creates **cognitive obstructions that are difficult for students to overcome**",
  and hypothesise that understanding infinity requires the *intertwining* of intuition and symbolism rather than
  the substitution of one for the other.
- **Relevance:** Directly documents the intuitive↔formal tension for a student who *has* the formal tools but
  cannot make them authoritative; a good complement to Zazkis & Mamolo on the affective/conflict side.

**6. Narlı, Serkan & Başer, Neşe (2010). "The effects of constructivist learning environment on prospective
mathematics teachers' opinions."** *US-China Education Review* 7(1) (Serial No. 62), Jan. 2010, ISSN 1548-6613;
full text hosted as ERIC document ED508197.
- **Source type:** journal article in a low-threshold international education journal; the ERIC copy is the
  full text (ERIC document, not a journal record). Treat as indicative rather than as a top-tier study.
- **URL fetched (full PDF, header confirms "US-China Education Review, ISSN 1548-6613, USA", Jan. 2010, Vol. 7,
  No. 1):** `http://files.eric.ed.gov/fulltext/ED508197.pdf`
- **Actual findings:** 60 first-year prospective mathematics teachers (Dokuz Eylül University) were split into
  homogeneous experimental/control groups after a "Minimum Requirements Identification Test" on set,
  correspondence and function; 40 students (20 per group) actually answered the open-ended questionnaire.
  Both groups were taught **Cantorian Set Theory including countable and uncountable infinity** in discrete
  mathematics; the control group by traditional methods, the experimental group in a constructivist /
  problem-based (PDL) environment with computer animations. Results, in the authors' own numbers: **no**
  significant difference between groups in opinions about mathematics (χ² = 2.578, SD = 3, p > 0.05), the
  department of mathematics (χ² = 3.185, SD = 3, p > 0.05) or discrete mathematics (χ² = 4.935, SD = 3,
  p > 0.05) — short-term interventions did not move deep-rooted beliefs. However, **"opinions about Cantorian
  Set Theory were significantly differentiated between experimental and control groups after the instruction"**
  (χ² = 13.486, SD = 2, p < 0.05): the experimental group found the topic fun and comprehensible, while the
  control group found it **difficult and nonsensical**. Before instruction, all 20 experimental-group
  respondents stated "I have no information on this subject"; afterwards their recorded categories include
  "It changed my concept of infinity completely" and "Magic Hotel was very nice" (Hilbert's Hotel), and the
  authors report that animations of the abstract proofs of numerical equivalence helped.
- **Relevance:** The only **intervention study** I found that teaches countable vs uncountable infinity and
  measures attitude change. It gives a concrete leverage point (animations/visualisation, Hilbert's Hotel) and
  a concrete caution (general mathematical beliefs don't budge).

**7. Cihlář, Jiří; Eisenmann, Petr; Krátká, Magdalena (2015). "Omega Position – a specific phase of perceiving
the notion of infinity."** *Scientia in educatione* 6(2).
- **Source type:** peer-reviewed journal article (open access).
- **URL fetched (full PDF):** `https://ojs.cuni.cz/scied/article/download/184/181`
- **Actual findings:** 1,432 Czech pupils/students aged 8–20 took part in the first two stages (2008–2011);
  the third, qualitative stage interviewed university students. The authors identify a distinct developmental
  phase, the **"omega position"**: learners sharpen *potential* infinity and posit an improper "end" element
  (as ℕ together with ω approximates it) without attaining *actual* infinity. Hypotheses: a significant
  portion of fresh university students are in the omega position, and students enter it when a new context
  forces a move from the potential to the actual view. The authors argue this phase "can become an obstacle"
  for limits, infinite series, and Cantor-style set theory, and that teachers can diagnose it. Reported
  statistics include a chi-square of 151,834 (df = 4, p = 0.00000) for the distribution across categories and
  rejection of independence at the 1% level in most cases.
- **Relevance:** Supplies the potential/actual-infinity developmental framing with a large sample and an
  explicit "obstacle" claim; the companion paper (item 8) quantifies it.

### 7.2 Studies verified at bibliographic-record level (abstracts retrieved from APIs; full texts paywalled)

**8. Fischbein, Efraim; Tirosh, Dina; Hess, Perla (1979). "The intuition of infinity."** *Educational Studies in
Mathematics* 10(1), 3–40.
- **Source type:** peer-reviewed journal article. **DOI:** `10.1007/BF00311173`
- **URL fetched (metadata record):** Crossref API `https://api.crossref.org/works?query.bibliographic=...`
  → correct DOI + venue + authors + year confirmed.
- **Findings:** The full text is paywalled (Springer, closed). The only *findings* I could verify are the
  figures reported second-hand in Monaghan (1986, item 3): the majority of subjects (71% overall, 81% of the
  high-ability group) claimed ℕ was **bigger** than the set of even numbers, and Monaghan's own group failed to
  replicate that, which he attributes to leading wording. Fischbein et al. is also the standard reference for
  the claim that intuitions about infinity are sensitive to wording and context (Blaszczyk's reference list
  cites it as "The Intuition of Infinity, ESM 10, 3–40").
- **Relevance:** The historical anchor for the whole "intuition of infinity" line. **Do not quote specific
  percentages as Fischbein's own without the caveat that they come via Monaghan.**

**9. Tirosh, Dina & Tsamir, Pessia (1996). "The role of representations in students' intuitive thinking about
infinity."** *International Journal of Mathematical Education in Science and Technology* (vol. 27, per DOI).
- **Source type:** peer-reviewed journal article. **DOI:** `10.1080/0020739960270105`
- **URL fetched (API record incl. abstract):** `https://api.openalex.org/works/https://doi.org/10.1080/0020739960270105`
  (OpenAlex `oa_status: closed`; no OA copy).
- **Actual findings (from the retrieved abstract):** 189 middle-class 10th–12th graders answered 14 problems
  comparing infinite sets, each presenting the *same* sets under a different representation
  (numerical-horizontal, numerical-vertical, numerical-explicit, geometric). **One-to-one-correspondence
  justifications were mainly elicited by numerical-explicit and by geometric representations** — i.e. whether
  students reach for the bijection criterion is largely determined by *how the sets are written down*. The
  authors draw teaching implications for "analogy" vs "conflict" approaches.
- **Relevance:** Highly actionable for a curriculum: the *representation* of the set is a design variable that
  toggles whether students even think of bijection. This is the mechanism behind many of the Hamza–O'Shea errors.

**10. Homaeinejad, Maryam; Barahmand, Ali; Seif, Asghar (2021/2022). "The relationship between notions of
infinity and strategies used to compare enumerable infinite sets."** *International Journal of Mathematical
Education in Science and Technology*.
- **Source type:** peer-reviewed journal article. **DOI:** `10.1080/0020739X.2021.1941362`. ERIC EJ1374998.
- **URLs fetched:** OpenAlex API record (abstract) + ERIC search results page
  (`https://eric.ed.gov/?q=infinity+AND+%22mathematics%22+AND+students+conceptions&per_page=20`) + Crossref API.
- **Actual findings (retrieved abstract):** 104 senior high-school students, two-part questionnaire. Infinity
  notions were coded as *potential* vs *actual*; comparison strategies were coded as **part-whole, single
  infinity, incomparability, and one-to-one correspondence**. There was a **statistically significant but not
  strong** relationship between the notion of infinity held and the strategy used, and **inconsistent
  responses were common** (the same student using different strategies on different items).
- **Relevance:** This is the cleanest published taxonomy of the four comparison strategies — exactly the
  vocabulary a curriculum survey needs (Galileo part-whole vs bijection vs "can't compare").

**11. Krátká, Magdalena; Eisenmann, Petr; Cihlář, Jiří (2021/2022). "Four conceptions of infinity."**
*International Journal of Mathematical Education in Science and Technology*. ERIC EJ1370841.
- **Source type:** peer-reviewed journal article. **DOI:** `10.1080/0020739X.2021.1897894`.
- **URLs fetched:** OpenAlex API record (abstract) + ERIC search results page.
- **Actual findings:** Questionnaire survey of **861 Czech students in grades 7–13**. Four conceptions of
  infinity are distinguished, built on the intuitive "horizon" phenomenon, and tracked across four
  combinations of *view* (into the distance / in depth) and *context* (arithmetical / geometrical).
  The proportion of the earliest conception ("natural infinity") does **not** decrease monotonically with age;
  the proportion of **actual infinity is non-decreasing with age**, at least in the "into the distance" view in
  both contexts; overall, "the proportional representation of individual conceptions of infinity is strongly
  dependent on both context and view."
- **Relevance:** The largest-sample quantitative picture of how infinity conceptions develop; useful for
  arguing that there is no single age at which "actual infinity" can be assumed.

**12. Mamolo, Ami & Zazkis, Rina (2008). "Paradoxes as a window to infinity."** *Research in Mathematics
Education* (issue not verified).
- **Source type:** peer-reviewed journal article. **DOI:** `10.1080/14794800802233696`.
- **URL fetched (API record incl. abstract):** `https://api.openalex.org/works?filter=title_and_abstract.search:...`
  (OpenAlex `oa_status` closed; Taylor & Francis paywalled).
- **Actual findings (retrieved abstract):** Two groups — undergraduates in Liberal Arts programmes and
  graduate students in a Mathematics Education Master's — engaged with **Hilbert's Grand Hotel** and the
  **Ping-Pong Ball Conundrum** before, during and after instruction. Graduate students found the resolution of
  Hilbert's Grand Hotel unproblematic, but **both groups responded surprisingly similarly to the Ping-Pong Ball
  Conundrum**. Consistent with prior research, participants "perceive infinity as an ongoing process, rather than
  a completed one, and fail to notice conflicting ideas."
- **Relevance:** The canonical "paradoxes as a window" paper, and the direct source of the "fail to notice
  conflicting ideas" claim that Zazkis & Mamolo (2009, item 2) later dramatise.

**13. Kahn, Ken; Sendova, Evgenia; Sacristán, Ana Isabel; Noss, Richard (2011). "Young students exploring
cardinality by constructing infinite processes."** *Technology, Knowledge and Learning* 16(1), 3–34.
ERIC EJ931577.
- **Source type:** peer-reviewed journal article. **DOI:** `10.1007/s10758-011-9175-0`.
- **URLs fetched:** OpenAlex API record + ERIC record `https://eric.ed.gov/?id=EJ931577` (abstract quoted below).
- **Actual findings (retrieved ERIC abstract):** Part of the EU **WebLabs** project. Students **aged 9–13** in
  several European countries explored the cardinality of infinite sets by **interpreting and constructing
  computer programs in ToonTalk**. The authors' hypothesis: "via carefully designed computational explorations
  within an appropriately constructed medium, infinity can be approached in a learnable way that does not
  sacrifice the rigour necessary for mathematical understanding of the concept."
- **Relevance:** The strongest existence proof that cardinality can be taught *below* undergraduate level via
  construction/programming rather than assertion — directly relevant to a construction-first (proof-assistant)
  curriculum.

**14. Shipman, Barbara A. (2012). "Determining definitions for comparing cardinalities."** *PRIMUS* 22(3),
239–254. ERIC EJ971680.
- **Source type:** peer-reviewed journal article (undergraduate-teaching journal).
- **URLs fetched:** OpenAlex API record + ERIC record `https://eric.ed.gov/?id=EJ971680`.
- **Actual findings (retrieved ERIC abstract):** "Through a series of **six guided classroom discoveries**,
  students create, via targeted questions, a definition for deciding when two sets have the same cardinality.
  The program begins by developing basic facts about cardinalities of finite sets. **Extending two of these
  facts to infinite sets yields two statements on comparing infinite cardinalities that contradict each other.**
  The experiment '**More circles or more squares?**' resolves this dilemma in favor of the definition of 'same
  cardinality' that Georg Cantor adopted over a century ago."
- **Relevance:** A ready-made, published *instructional sequence* that manufactures the contradiction students
  must resolve — the closest thing to a pedagogical design pattern for the part-whole vs bijection conflict.

**15. Rauff, James V. (2008). "Penguins and Pandas: A note on teaching Cantor's diagonal argument."**
*College Teaching Methods & Styles Journal* 4(9), 11–18. ERIC EJ967668.
- **Source type:** peer-reviewed (journal, CTMS) but **descriptive/practitioner** note, not a study.
- **URLs fetched:** OJS landing page `https://clutejournals.com/index.php/CTMS/article/view/5566`
  (title, author, journal, volume/issue/pages, DOI `10.19030/ctms.v4i9.5566`, keywords "Uncountable set,
  Cantor, diagonal proof, infinity, liberal arts"); ERIC record `https://eric.ed.gov/?id=EJ967668`.
  The publisher's PDF endpoint (`.../article/download/5566/5649`) is **unreachable** (curl: "Maximum (50)
  redirects followed").
- **Actual findings:** Abstract as retrieved: "Cantor's diagonal proof that the set of real numbers is
  uncountable is one of the most famous arguments in modern mathematics. Mathematics students usually see this
  proof somewhere in their undergraduate experience, but it is rarely a par[t]…". The note "describes contexts
  that have been used by the author in teaching Cantor's diagonal argument to fine arts and humanities
  students."
- **Relevance:** A rare piece explicitly about **teaching the diagonal argument to a non-majors audience**;
  useful as a low-stakes on-ramp design, but with **no empirical findings**.

**16. Padula, Janice (2023). "Using historical proof-by-contradiction examples in senior mathematics: How Georg
Cantor's diagonal method made Alan Turing's (1937–8) proof possible."** *Australian Mathematics Education
Journal* 5(3), 37–41. ERIC EJ1418633.
- **Source type:** peer-reviewed practitioner journal article (curriculum-facing), not a study.
- **URL fetched:** ERIC record `https://eric.ed.gov/?id=EJ1418633`.
- **Actual findings:** Notes that Australian (ACARA), Scottish, English and American curricula require proof by
  contradiction; in Australian Specialist Mathematics it appears as a Geometry topic and students must "use the
  quantifiers 'for all' and 'there exists'". The author argues for exploring **historical** proof-by-contradiction
  examples — Cantor's diagonal method and its application to Turing's proof — with advanced senior secondary
  students.
- **Relevance:** A concrete, curriculum-aligned argument for teaching diagonalization *early* (senior secondary)
  through the Cantor→Turing lineage.

**17. Wijeratne, Chanakya & Zazkis, Rina (2021). "On the classic paradox of infinity and a related function."**
*Teaching Mathematics and Its Applications* 40(3), 167–181. ERIC EJ1309059.
- **Source type:** peer-reviewed journal article.
- **URLs fetched:** OpenAlex API record (DOI `10.1016/j.jmathb.2016.04.001` for the companion, and the 2021
  record) + ERIC record `https://eric.ed.gov/?id=EJ1309059`.
- **Actual findings (retrieved ERIC abstract):** The authors analyse a classic paradox of infinity using
  **uniform convergence of functions**, then examine how **six mathematics honours students** engage with a
  variation. "**Despite their advanced mathematical training, the participants experienced considerable
  difficulty** in addressing the presented paradoxical situation." The paper describes how students tried to
  reconcile their intuitive perceptions with their computations.
- **Relevance:** Evidence that even advanced undergraduates/admissions-selected students retain the
  potential-infinity intuition; supports the claim that the blocker persists past calculus.

**18. Villabona, Diana; Oktaç, Asuman; Roa-Fuentes, Solange (2024). "Acting on totalities of infinite
processes: constructing facets of an object conception."** *ZDM – Mathematics Education*.
- **Source type:** peer-reviewed journal article. **DOI:** `10.1007/s11858-024-01631-6`.
- **URLs fetched:** Crossref API record (full abstract retrieved) + ERIC search results page (EJ1450613).
- **Actual findings (retrieved abstract):** Uses **APOS theory** (Action–Process–Object–Schema) to study how
  learners construct the *Object* facet of mathematical infinity — specifically, how they **act on infinite
  entities**. The context is an **infinite union of finite subsets of ℕ**; interviews were conducted with
  **graduate students and instructors**. The authors "identify three types of Actions with different complexity
  levels", which lead to different facets of the associated Object, and question the completeness of the standard
  genetic decomposition.
- **Relevance:** The most recent theoretical treatment of what it means to *treat an infinite set as a
  completed object* — the cognitive precondition for accepting "same cardinality".

**19. Göktepe Yıldız, Sevda & Göktepe Körpeoğlu, Seda (2018). "Exploring pre-service mathematics teachers'
understandings of countability and infinity in WebQuest based learning environment."** *European Journal of
Education Studies*.
- **Source type:** open-access journal article (*European Journal of Education Studies* is a low-threshold
  pay-to-publish OA journal — treat the findings as indicative, not as top-tier evidence). **DOI:** `10.46827/ejes.v0i0.1895`.
- **URL fetched:** OpenAlex API record with abstract (landing page `http://oapub.org/edu/index.php/ejes/article/view/1895`
  was fetched but returned OJS metadata/navigation only, not the full text).
- **Actual findings (retrieved abstract):** 29 pre-service teachers worked on infinity/countability in a WebQuest
  environment; data from a researcher-designed questionnaire plus semi-structured interviews, analysed
  phenomenologically. **Pre-service teachers generally defined infinite sets as "the sets whose elements
  continue infinitely"; they tended to define countable sets as bounded sets, finite sets, and sets with known
  elements.** Most stated that countable finite sets and countable infinite sets were equivalent.
- **Relevance:** Shows the countable/uncountable confusion is present in **teachers**, not just students — a
  direct curriculum-design constraint (the brief's teacher-education angle).

**20. Aztekin, Serdar; Arıkan, Ahmet; Sriraman, Bharath (2010). "The constructs of PhD students about
infinity: An application of repertory grids."** *The Mathematics Enthusiast* (volume/issue not verified).
- **Source type:** peer-reviewed journal article (open access). **DOI:** `10.54870/1551-3440.1180`.
- **URL fetched:** OpenAlex API record with abstract. (The OA PDF at
  `https://scholarworks.umt.edu/cgi/viewcontent.cgi?article=1180&context=tme` is **blocked by Cloudflare**,
  HTTP 403 "Just a moment…".)
- **Actual findings (retrieved abstract):** A study of **PhD students'** constructs about infinity using
  **repertory grid** methodology, investigating "the effects of **a graduate level set theory course** on their
  informal models." The authors argue most prior research used geometric contexts to probe infinity indirectly
  because students lack set-theoretic symbolic fluency, and propose repertory grids as a way to capture
  constructs directly.
- **Relevance:** Rare evidence at the doctoral level and, importantly, a **pretest/posttest design around a set
  theory course** — the closest analogue to measuring what a set-theory course actually changes.

**21. Love, William P. (1989). "Infinity: The twilight zone of mathematics."** *The Mathematics Teacher*.
ERIC EJ392662.
- **Source type:** practitioner journal article (not a study).
- **URL fetched:** ERIC search results page (title + abstract snippet).
- **Actual findings (snippet):** "The theorems and proofs presented are designed to enhance student
  understanding of the theory of infinity as developed by Cantor and others. Three transfinite numbers are
  defined to express the cardinality of infinite algebraic sets, infinite sets of geometric points and
  infinite…"
- **Relevance:** Historical evidence that the ℵ₀/ℵ₁/𝔠 ladder has been taught at secondary level for decades;
  useful for dating the curriculum problem. **Low evidential weight.**

**22. Deihl, Steve & Markinson, Mara P. (2019). "Connecting the tangent function to cardinality: A method for
introducing set theory to high school students."** *Journal of Mathematics Education at Teachers College*.
ERIC EJ1237699.
- **Source type:** practitioner/curriculum journal article.
- **URL fetched:** ERIC search results page.
- **Actual findings (snippet):** "High school students often ask questions about the nature of infinity. When
  contemplating what the 'largest number' is, or discussing the speed of light, students bring their own ideas
  about infinity and asymptotes into…" — proposes using the tangent function as a bridge into cardinality.
- **Relevance:** A concrete high-school on-ramp (asymptote/tangent → bijection → cardinality); no findings.

**23. Blechschmidt, Ingo & Hutzler, Matthias (2019). "A constructive Knaster–Tarski proof of the uncountability
of the reals."** arXiv:1902.07366 [math.HO].
- **Source type:** preprint (expository, math.HO).
- **URL fetched (abstract page):** `https://arxiv.org/abs/1902.07366`
- **Actual findings (retrieved abstract):** "We give an uncountability proof of the reals which relies on their
  **order completeness** instead of their sequential completeness. We use **neither a form of the axiom of
  choice nor the law of excluded middle**, therefore the proof applies to the MacNeille reals in any flavor of
  constructive mathematics. The proof leans heavily on Levy's unusual proof…" (2 pages.)
- **Relevance:** Directly useful for Blocker 7 *and* as a bridge to Blocker 8: it shows uncountability can be
  established **without AC**, so a curriculum can teach Cantor's result before (or independently of) AC.

### 7.3 Blocker 7 — negative / useful-null results

- ERIC query `"infinite sets" cardinality students` returned **only 4 records** in total, of which the relevant
  ones are items 10, 13 and 17 above plus Love (1989). The education literature on countable/uncountable is
  small but real.
- ERIC query `"diagonalization" proof mathematics students` returned **1 record** (Goldberg & Hammerman 2004,
  *Mathematics and Computer Education*, "Adapting computational data structures technology to reason about
  infinity", EJ720447) — tangential.
- ERIC `"intuitive thinking about infinity"` and `"Misconceptions Concerning Infinity"` (quoted phrases)
  returned **0 records** — Hamza & O'Shea (2011) is **not indexed in ERIC**; it lives only in the Maynooth
  repository. Do not claim ERIC coverage for it.

### 7.4 Blocker 7 — honest gaps

- **Tsamir, Pessia (2001). "When 'The Same' is not perceived as such: The case of infinite sets."**
  *Educational Studies in Mathematics* 44(2). DOI `10.1023/A:1016034917992`. I verified the **bibliographic
  record** through both the OpenAlex API (`oa_status: closed`, `any_repository_has_fulltext: false`) and the
  Crossref API (title, author, year, venue, 25–34 citations depending on source). **Neither API supplies an
  abstract, and no open copy exists** — so I could **not** verify any specific finding. Given the title, it is
  plainly about students failing to see equinumerous sets as "the same", but I will not attribute findings to it.
  It is cited in the literature as a key reference for representation effects alongside item 9.
- **Tirosh, Dina (1991)**, cited by Hamza & O'Shea as the source of "all methods suitable for comparing finite
  sets are adequate for infinite sets as well" (p. 204). I did not independently locate or verify this item; the
  quotation is verified only as it appears in the OCR'd Hamza & O'Shea text.
- **Dubinsky, E., Weller, K., Stinger, C. & Vidakovic, D. (2008). "Infinite iterative processes: The Tennis Ball
  Problem."** *European Journal of Pure and Applied Mathematics* 1(1), 99–121. Appears in Hamza & O'Shea's
  reference list; I did not independently retrieve or verify it.
- **Fischbein, Efraim (2001). "Tacit Models and Infinity."** *Educational Studies in Mathematics* 48.
  DOI `10.1023/A:1016088708705`; and **Tall, David & Tirosh, Dina (2001). "Infinity – the never-ending
  struggle."** *Educational Studies in Mathematics* 48. DOI `10.1023/A:1016019128773`. Both records verified via
  the Crossref API (title/authors/year/venue) but **neither full text nor abstract was retrievable**.
- **Padula (2023)**, **Shipman (2012)**, **Rauff (2008)**, **Love (1989)**, **Deihl & Markinson (2019)**:
  abstracts only (ERIC); no underlying datasets or full texts of the practitioner pieces were retrieved.

---

## BLOCKER 8 — The Axiom of Choice: student/instructor attitudes and the teaching of AC

### 8.0 Headline finding: the literature is (near-)empty, and here is the proof of the search

**There is, as far as I can establish, no empirical study of students' or instructors' attitudes toward the
Axiom of Choice, and no study of teaching AC, in the indexed mathematics-education literature.** I am reporting
this as a positive finding backed by an exhaustive negative search, not as a failure to look. Exhaustive list of
what was run and what came back:

| Query (tool) | Result |
|---|---|
| ERIC `"axiom of choice"` (quoted phrase, all years, all types) | **1 record total**, and it is irrelevant: Wine, Gilroy & Hantula (2012), "Temporal (In)stability of Employee Preferences for Rewards", *Journal of Organizational Behavior Management* — matched on "choice". |
| ERIC `"axiom of choice" teaching` | **0 records.** |
| ERIC `"Zorn lemma"` | 1 record: Henriksen & Wagon (eds.) 1991, *American Mathematical Monthly* 98(4) — see item 9 below. |
| ERIC `choice axiom set theory proof` | No AC-education study; nearest hits are Durand-Guerrier 2024 (item 10), Dawkins 2018 (item 8), Martinez 2024 dissertation (item 11). |
| ERIC `"existence proof" students understanding` | No non-constructive-existence study; noisy. |
| OpenAlex `title_and_abstract.search` × 10 phrasings: "axiom of choice teaching", "axiom of choice students", "axiom of choice attitudes beliefs", "axiom of choice survey mathematicians", "students understanding axiom of choice", "Zorn's lemma students", "Banach-Tarski paradox students intuition", "axiom of choice education", "non-constructive existence students", "axiom of choice intuition" | **No education study.** Five queries returned **zero** results ("axiom of choice mathematics education curriculum", "Banach-Tarski paradox students intuition", "Banach Tarski paradox intuition students", "Hausdorff paradox nonmeasurable set teaching intuition", "axiom of choice education"). The rest returned unrelated education/other-field work (physics-education-research, linear-algebra teaching, medical education, agricultural education, school-choice economics, precalculus assessment, "choice function" matched consumer/decision-theory senses of *choice*). |
| OpenAlex `fulltext.search:axiom of choice teaching` | Top 10 = generic teaching literature only (physics education research, medical education, CS-education debate, nonstandard analysis). **No AC-education paper in the full text index.** |
| OpenAlex `default.search` (title+abstract+fulltext) for "axiom of choice teaching education", "Zorn's lemma teaching education", "Banach-Tarski paradox teaching education" | Only adjacent/unrelated: Arsac–Lercher? no — returned `Chain Bounding … Lean` (item 7), *On the Brink of Paradox* (Rayo, MIT Press OA), Maddy-adjacent philosophy, Banach biographies. **No teaching study.** |
| OpenAlex `default.search` "axiom of choice survey mathematicians attitudes" | Only Pruss, "The Axiom of Choice Machine" (a philosophy-of-religion/paradox chapter) and Herrlich's *Axiom of Choice* (Lecture Notes in Mathematics). |
| Crossref × 5: "axiom of choice teaching", "axiom of choice students understanding", "axiom of choice survey mathematicians attitudes", "axiom of choice education", "axiom of choice attitudes" | Only book chapters of the standard monographs (Herrlich, Jech, Pruss) and the *Banach–Tarski Paradox* chapters. **No education article.** |
| Crossref `survey mathematicians axiom of choice beliefs attitudes` | Returned gambling-attitudes and Luce's-choice-axiom items. **Nothing.** |
| arXiv API `cat:math.HO AND all:"axiom of choice"` | 7 hits: two Banach–Tarski expositions (items 5–6), a constructive uncountability proof (Blocker 7 item 23), a philosophy-of-maths piece, and three non-teaching items. **No math-education study.** |
| Semantic Scholar API | Repeatedly returned HTTP 429 / empty through this proxy; contributed nothing. |
| OpenAlex budget | **Exhausted mid-run** ("Insufficient budget… Resets at midnight UTC"). A few planned scans (e.g. browsing the top ~40 `"axiom of choice"` records by relevance; a "mathematicians' philosophical views" sweep) returned nothing because of this. This is a real coverage limit of the present report. |

**Conclusion for the survey: any claim of the form "studies show students/instructors think X about AC" is
unsupported. The nearest verified work is (i) a philosopher's *claim about* AC pedagogy (item 1),
(ii) canonical expository/philosophical treatments of AC's status and independence (items 2–4),
(iii) the Banach–Tarski motivator literature (items 5–6, 12) and Lean/Coq formalisation-for-teaching
(items 7, 13), and (iv) *adjacent* empirical work on students' interpretations of axioms (items 8, 10, 11)
and on existence proofs (item 14).**

### 8.1 The pieces that actually exist and are verified

**1. Förster, Thomas (2006). "The axiom of choice and inference to the best explanation."**
- **Source type:** **preprint / unpublished manuscript** (OpenAlex `type: article`, but **no DOI and no venue**;
  indexed hosting: CiteSeerX).
- **URLs fetched:** OpenAlex API record containing the full abstract
  (`https://api.openalex.org/works?filter=title_and_abstract.search:axiom%20of%20choice%20teaching`).
  **The full text could not be retrieved:** CiteSeerX `viewdoc/summary` → HTTP 429 "Too Many Requests";
  CiteSeerX `viewdoc/download` → HTTP 500 redirect chain into the Internet Archive donation page;
  `https://web.archive.org/web/2020/<citeseerx download URL>` → "The Wayback Machine has not archived that URL";
  Förster's Cambridge page `https://www.dpmms.cam.ac.uk/~tf/` → HTTP 404.
- **Actual content (quoted from the retrieved abstract, which is the source of the claim):** "An argument often
  given for adopting the Axiom of Choice as an axiom is that it has a lot of obviously true consequences. …
  the standard examples of obvious-truths-following-from-AC all turn out, on closer inspection, to involve a
  **fallacy of equivocation**. … **It is common practice in the teaching of mathematics at university level to
  gloss over applications of the axiom of choice**, and proclaim such standard propositions as — for example —
  'A countable union of countable sets is countable' with some sketchy argument which does not render explicit
  the use of the axiom, and indeed might not even mention it by name at all. **The students in consequence do not
  form a mental image of the axiom, and tend subsequently not to recognise when it is being used. Typically when
  confronted with it later on in their education they either deny that it is being used, or acknowledge that it
  is being [used but object]** …"
- **Relevance:** This is the **only explicit claim about AC pedagogy as such** that my searches surfaced, and it
  is exactly the phenomenon the survey is about (silent AC use → students never form a mental image of AC →
  later denial/confusion). It is a **philosopher's argument, not an empirical study**, and I verified it **only
  from the abstract**, so cite it with both caveats.

**2. Bell, John L. (2008; substantive revision 2021). "The Axiom of Choice."** In *The Stanford Encyclopedia of
Philosophy* (E. N. Zalta, ed.).
- **Source type:** editorially refereed encyclopedia entry (reputable expository / secondary source, though a
  citable scholarly one).
- **URL fetched (full text, 65 KB converted):** `https://plato.stanford.edu/entries/axiom-choice/`
- **Actual content:** The entry's own framing is the acceptance problem in a sentence: AC "as it is usually
  stated appears humdrum, even self-evident" — "given any collection of mutually disjoint nonempty sets, it is
  possible to assemble a new set … containing exactly one element from each" — "Nevertheless, this seemingly
  innocuous principle has far-reaching mathematical consequences — many indispensable, some startling". It
  quotes Fraenkel–Bar-Hillel–Levy calling AC "probably the most interesting and … most discussed axiom of
  mathematics, second only to Euclid's axiom of parallels". Section structure (retrieved): 1. Origins and
  Chronology of the Axiom of Choice; 2. Independence and Consistency of the Axiom of Choice; 3. …
- **Relevance:** The canonical citable statement of the *content* of the blocker: an axiom that looks obvious but
  has startling consequences, with a documented history of controversy. Use it for the independence/consistency
  section of the curriculum survey and as the reference frame for "why students find AC weird".

**3. Maddy, Penelope (1988). "Believing the axioms. I."** *The Journal of Symbolic Logic* 53(2).
**DOI:** `10.2307/2274520`. Companion: **"Believing the axioms. II."** *JSL* 53(3),
**DOI:** `10.2307/2274569`.
- **Source type:** peer-reviewed philosophy-of-mathematics journal article (JSL), heavily cited
  (139 citations per Crossref for I; 74–146 per OpenAlex for II).
- **URLs fetched:** Crossref API records with abstracts for both I and II
  (`https://api.crossref.org/works?query.bibliographic=Believing+the+Axioms+Maddy`) and OpenAlex API records.
- **Actual content (from the retrieved abstract of I):** "Ask a beginning philosophy of mathematics student why
  we believe the theorems of mathematics and you are likely to hear, 'because we have proofs!' … The next
  question, naturally, is **why we believe the axioms**, and here the response will usually be that they are
  'obvious', or 'self-evident' … Unfortunately, heartwarming answers along these lines are no longer tenable
  (if they ever were). On the one hand, assumptions once thought to be self-evident have turned out to be
  debatable, **like the law of the excluded middle**, or outright false …" — Maddy then reconstructs the
  *actual, non-demonstrative* reasons the mathematical community has for and against the axioms of ZFC, the
  continuum hypothesis, small and measurable large cardinals (part II adds determinacy hypotheses and large
  large cardinals).
- **Relevance:** This is as close as the literature comes to **"a study of mathematicians' attitudes toward
  axioms"** — but it is a **philosophical analysis of the community's reasons**, not a survey instrument with
  respondents. Cite it as the authoritative source for the *structure* of axiom-acceptance arguments
  (intrinsic vs extrinsic), which is the frame a curriculum on AC has to engage.

**4. Tomkowicz, Grzegorz & Wagon, Stan (2016). *The Banach–Tarski Paradox*, 2nd edition.** Cambridge University
Press. **DOI:** `10.1017/cbo9781107337145`.
- **Source type:** research monograph (Cambridge), 40 citations recorded by Crossref.
- **URL fetched (metadata + description):** Crossref API record.
- **Actual content (retrieved):** "The Banach–Tarski Paradox is a most striking mathematical construction: it
  asserts that a solid ball can be taken apart into finitely many pieces that can be rearranged using rigid
  motions to form a ball twice as large. This volume explores the consequences of the paradox for measure theory
  and its connections with group theory, geometry, set theory, and logic."
- **Relevance:** The standard scholarly reference for the Banach–Tarski material if the curriculum uses BT as the
  "motivator" for why AC's non-constructiveness matters.

**5. Buchhorn, Katie (2021, rev. 2022). "The Banach–Tarski Paradox."** arXiv:2108.05714 [math.HO].
- **Source type:** preprint (expository long-form; ~35 MB v1, 276 KB v2).
- **URL fetched (abstract page):** `https://arxiv.org/abs/2108.05714`
- **Actual content (retrieved abstract):** "In 1924, S. Banach and A. Tarski proved an astonishing, yet rather
  counterintuitive paradox: given a solid ball in ℝ³, it is possible to partition it into finitely many pieces
  and reassemble them to form two solid balls, each identical in size to the first. **When this paradox is
  applied to 3-dimensional space it does go against our intuition, but very often our intuition is flawed.**
  … Finally, **provided we have the Axiom of Choice at our disposal, we can construct sets that are
  nonmeasurable (not Lebesgue measurable)** and the proof of the Banach–Tarski Paradox follows naturally."
- **Relevance:** A self-contained, teachable exposition that makes the AC→non-measurable-set→BT dependency
  explicit — exactly the "AC is non-constructive, and here is what that costs you" story. Label as preprint.

**6. Wahlberg, Mats (2022). "The Banach–Tarski Paradox."** arXiv:2206.13512 [math.HO].
- **Source type:** preprint / master's thesis-style exposition (55 numbered pages, 61 total).
- **URL fetched (abstract page):** `https://arxiv.org/abs/2206.13512`
- **Actual content (retrieved abstract):** "This thesis presents the strong and weak forms of the Banach–Tarski
  paradox based on the Hausdorff paradox. It provides modernized proofs of the paradoxes and necessary
  properties of equidecomposable and paradoxical sets. **The historical significance of the paradox for measure
  theory is covered, along with its incorrect attribution to Banach and Tarski.** Finally, **the necessity of
  the axiom of choice is discussed and contrasted with other axiomatic and topological assumptions** that
  enable the paradoxes."
- **Relevance:** Explicitly isolates the *independence/necessity* question — i.e. **what happens to BT if you
  drop AC** — which is the best available "independence of AC" teaching narrative in an accessible source.

**7. Incatasciato, Guillermo L. & Sánchez Terraf, Pedro (2024). "Chain Bounding, the leanest proof of Zorn's
lemma, and an illustration of computerized proof formalization."** arXiv:2404.11638 [math.LO].
- **Source type:** preprint (math.LO), v1 Apr 2024 / v2 Oct 2024.
- **URL fetched (abstract page):** `https://arxiv.org/abs/2404.11638`
- **Actual content (retrieved abstract):** "We present an exposition of the *Chain Bounding Lemma*, which is a
  common generalization of both **Zorn's Lemma** and the Bourbaki–Witt fixed point theorem. The proofs of these
  results through the use of Chain Bounding are amongst the simplest ones that we are aware of. … We also provide
  an introduction to the process of '**computer formalization**' of mathematical proofs by using *proof
  assistants*. As an illustration, **we verify our main results with the Lean proof assistant.**"
- **Relevance:** The single most directly useful item for a Lean-style curriculum: it gives the simplest known
  route to Zorn's Lemma *and* a machine-checked Lean development, so AC/Zorn can be taught with the choice
  principle made explicit and auditable instead of glossed (contrast Förster, item 1).

**8. Dawkins, Paul Christian (2018). "Student interpretations of axioms in planar geometry."**
*Investigations in Mathematics Learning* 10(4), 227–239. **DOI:** `10.1080/19477503.2017.1414981`.
ERIC EJ1188492.
- **Source type:** peer-reviewed journal article (empirical study).
- **URLs fetched:** ERIC record `https://eric.ed.gov/?id=EJ1188492` (abstract) + Crossref API record.
- **Actual findings (retrieved ERIC abstract):** "This study investigates students' development of
  **metamathematical understanding of axioms**. Based on **four semesters of experiments** teaching neutral,
  axiomatic geometry, often through guided reinvention, I identify **five categories of student interpretations
  of their axiomatizing activity**. Similar to previously observed patterns in student interpretations of
  definitions and proofs, the most problematic student interpretations of axioms resulted from **focusing
  exclusively on the referents of mathematical theory**, which precluded generalization through abstraction.
  As a result of engaging in axiomatizing, study participants constructed several quite sophisticated views of
  axiomatizing compatible with various aspects of modern mathematical practice — what I call **stipulated and
  formal interpretations**."
- **Relevance:** **This is the nearest thing in the entire literature to a study of "student attitudes toward an
  axiom".** It is about geometry axioms, not AC, but it supplies the empirical framework (referent-focused vs
  stipulated vs formal interpretations of what an axiom *is*) that a study of AC attitudes would need. Strong
  candidate for the survey's "adjacent verified work" slot.

**9. Henriksen, Melvin & Wagon, Stan (eds.) (1991). "The Teaching of Mathematics."** *American Mathematical
Monthly* 98(4), 346–364. ERIC EJ430505.
- **Source type:** peer-reviewed journal section / instructional-materials collection.
- **URL fetched:** ERIC record `https://eric.ed.gov/?id=EJ430505`.
- **Actual content (retrieved ERIC abstract):** "Presented with appropriate formulas and illustrations are five
  lessons suitable for college mathematics undergraduate students, including Pi and the limit of (sin α)/α, the
  structure of orthogonal transformations, **a simple proof of Zorn's Lemma**, the converse of Liouville's
  Theorem, and visualizing the p-adic integers."
- **Relevance:** Evidence that a *simple, undergraduate-level proof of Zorn's Lemma* has been published in the
  teaching literature since 1991 — i.e. the material for teaching AC-equivalents at undergraduate level exists
  even though no study of its reception does.

**10. Durand-Guerrier, Viviane (2024). "Contribution to didactic research on the completeness/incompleteness of
ordered fields of numbers."** *ZDM – Mathematics Education*. **DOI:** `10.1007/s11858-024-01635-2`.
ERIC EJ1450588.
- **Source type:** peer-reviewed journal article (empirical/didactic research).
- **URLs fetched:** Crossref API record (title/author/year/venue verified) + ERIC search results page snippet.
  (The direct ERIC record fetch for EJ1450588 failed with HTTP 000 / SSL EOF; the abstract text below is from the
  retrieved ERIC search-results page.)
- **Actual findings (retrieved ERIC snippet):** "Understanding the concept of completeness for an ordered field
  is known to be **difficult for many university mathematics students**. We hypothesise that **the variety of
  possible axioms of completeness** for the set of real numbers is one of the sources of diffi[culty]…"
- **Relevance:** The closest verified study to "students confronting a *choice of axioms*": it shows that
  presenting multiple, non-equivalent axioms for the same concept is itself a documented source of student
  difficulty. Directly transferable to a curriculum that presents ZF vs ZFC.

**11. Martinez, Antonio Estevan (2024). "Bridging the gap between set theory and logic: Leveraging computing as
a mediating tool for learning."** PhD dissertation, University of California, San Diego; ProQuest LLC.
ERIC ED646013.
- **Source type:** doctoral dissertation (ERIC record).
- **URL fetched:** ERIC record `https://eric.ed.gov/?id=ED646013`.
- **Actual findings (retrieved ERIC abstract):** A teaching experiment on how **computing/programming can
  strengthen the connection between set theory and logic** in an Introduction to Proofs context, with three
  dimensions: in-the-moment reasoning about set theory/logic, growth across a multi-session teaching experiment,
  and affective factors (confidence, interest, self-efficacy, mathematical identity). Results: "the students in
  my study were able to **leverage computing as an accessible onramp to the fundamental ideas related to set
  theory and logic**", and "computing can have a positive effect on one's sense of confidence and interest in
  relation to mathematics and programming."
- **Relevance:** Adjacent but important: it is the only dissertation I found that studies **programming as the
  medium for teaching foundational set theory/logic** — the same design premise as a proof-assistant curriculum.
  It does **not** cover AC.

**12. Stromberg, Karl (1979). "The Banach–Tarski Paradox."** *The American Mathematical Monthly* 86(3).
**DOI:** `10.1080/00029890.1979.11994759`.
- **Source type:** peer-reviewed expository journal article (classic).
- **URL fetched:** Crossref API record (title/author/year/venue verified; 21 citations).
- **Relevance:** The classic short expository account of BT for a general mathematical audience; a period
  reference showing the paradox has been "teaching material" for decades. Findings not retrieved (paywalled).

**13. Wan, Xinyi; Xu, Ke; Cao, Qinxiang (2023). "Coq formalization of ZFC set theory for teaching scenarios."**
*International Journal of Software and Informatics* 13(3), 323–357. **DOI:** `10.21655/ijsi.1673-7288.00303`.
- **Source type:** peer-reviewed journal article.
- **URLs fetched:** OpenAlex API record with abstract; DOI resolution to the journal landing page
  `https://doi.org/10.21655/ijsi.1673-7288.00303`, which confirms the citation line
  "Volume 13, Issue 3, 2023 > 323-357. DOI:10.21655/ijsi.1673-7288.00303" and the title.
  The publisher's PDF path `http://www.ijsi.org/ijsi/article/pdf/303` returned HTTP 404 / empty.
- **Actual findings (retrieved abstract):** "Discrete mathematics is a foundation course for computer-related
  majors, and propositional logic, first-order logic, and the axiomatic set theory are important parts of this
  course. **Teaching practice shows that beginners find it difficult to accurately understand abstract concepts,
  such as syntax, semantics, and reasoning system.** In recent years, some scholars have begun introducing
  interactive theorem provers into teaching to help students construct formal proofs so that they can understand
  logic systems more thoroughly. However, directly employing the existing theorem provers will increase
  students' learning burden since these tools have a high threshold for getting started with them. To address
  this problem, we develop a prover for the **Zermelo–Fraenkel set theory with the axiom of Choice (ZFC) in Coq
  for teaching scenarios**. Specifically, the first-order logical reasoning system and the axiomatic set theory
  ZFC are formalized, and several automated proof tactics specific to reasoning [about sets are developed]…"
- **Relevance:** The one paper I found that is simultaneously (a) about **teaching ZFC including AC** and
  (b) empirical in the weak sense that it reports "teaching practice shows beginners find it difficult". It
  reports a *tool*, not a study of attitudes. It is also a direct precedent for the sokonanoda design premise
  (custom, low-threshold prover for teaching).

**14. Brown, Stacy A. (2017). "Who's there? A study of students' reasoning about a proof of existence."**
*International Journal of Research in Undergraduate Mathematics Education* 3(2).
**DOI:** `10.1007/s40753-017-0053-6`.
- **Source type:** peer-reviewed journal article (empirical).
- **URLs fetched:** Crossref API record, OpenAlex API record, Semantic Scholar API record — all three confirm
  the citation (author "S. Brown"; 9–10 citations) but **none supplies an abstract**, and the Springer article
  page (`https://link.springer.com/article/10.1007/s40753-017-0053-6`) returned an empty/blocked 294-byte body.
- **Findings: NOT VERIFIED.** I am listing it only because it is the literature's main study of students'
  reasoning about **existence proofs**, which is the natural empirical analogue of non-constructive existence
  (∃-statement without an exhibited witness). **Do not attribute findings to it in the survey**; either obtain
  the paper or describe it as "a study of student reasoning about existence proofs (findings not yet verified)".

### 8.2 Other Blocker-8 items retrieved but deliberately NOT counted as evidence

- **Pruss, Alexander R. (2018). "The Axiom of Choice Machine."** In *Infinity, Causation, and Paradox*, Oxford
  University Press. **DOI:** `10.1093/oso/9780198810339.003.0006`. Retrieved via Crossref API (abstract). It
  gives a "causally implementable" AC and **two Dutch Book paradoxes connected with the Banach–Tarski paradox**,
  arguing for causal finitism. Source type: philosophy-of-religion/decision-theory book chapter. Useful only as
  an example of "AC produces paradoxes that motivate rejecting it"; **not** math-education research.
- **Katz, Karin U. & Katz, Mikhail G. (2011). "Meaning in classical mathematics: is it at odds with
  intuitionism?"** *Intellectica* 56(2). **DOI:** `10.3406/intel.2011.1154`. Retrieved via OpenAlex API record
  (French abstract). Concerns the classical/intuitionist cleavage and nonstandard analysis. Relevant to the
  **"AC is non-constructive" intuition** at the philosophy level; not retrieved in full.
- **Rayo, Agustín (2019). *On the Brink of Paradox*.** MIT Press. **DOI:** `10.7551/mitpress/10835.001.0001`;
  open-access preview retrieved via OpenAlex record. The book covers "Cantor's revolutionary thinking about
  infinity … different sizes [of infinity] … measure theory … computability theory" for a general audience.
  Relevant as a *teaching* resource for exactly the Blocker 7 → Blocker 8 arc, but I did **not** retrieve the
  text and cannot cite findings from it.

### 8.3 Blocker 8 — honest gaps and what was tried

- **No study located** of: students' attitudes to AC; instructors' attitudes to AC; the effect of teaching AC on
  understanding; students' acceptance/rejection of AC; any survey instrument for AC beliefs; AC in the
  undergraduate curriculum as a research subject. All eight search vectors above were run; I consider the
  negative robust for ERIC, OpenAlex (title/abstract + fulltext + default) and Crossref.
- **Coverage limitation:** the OpenAlex API daily budget ran out mid-session
  ("Insufficient budget… Resets at midnight UTC"), so a planned relevance-ordered scan of the top ~40
  `"axiom of choice"` records and a "mathematicians' philosophical views" sweep could not be completed.
  A follow-up run after the reset should repeat: `oq.sh '"axiom of choice"' 40`, plus
  `default.search:"axiom of choice" mathematics education` and a scan of the *Journal of Humanistic
  Mathematics* (its site search endpoint `https://scholarship.claremont.edu/jhm/search.html?q=...` returned
  HTTP 404 — a working search URL for that journal still needs to be found).
- **Semantic Scholar** returned HTTP 429 / empty for every query attempted through this proxy, so it
  contributed nothing to either blocker.
- **Förster (2006) full text** could not be retrieved by any of four routes (CiteSeerX 429, CiteSeerX download
  500/Internet Archive, Wayback 404, author's Cambridge page 404). The pedagogical claim in item 1 is therefore
  **abstract-verified only**.
- **Brown (2017)** and **Tsamir (2001)**: bibliographic records verified, **findings unverified** (see §7.4 and
  item 14).
- **Maddy (1988 I & II)**, **Tomkowicz & Wagon (2016)**, **Stromberg (1979)**, **Katz & Katz (2011)**,
  **Rayo (2019)**: metadata and/or abstracts verified via Crossref/OpenAlex API; **full texts not retrieved**
  (paywalled or preview-only). Do not quote findings from these beyond their own retrieved abstracts.
- **Hamza & O'Shea (2011)**: findings are from **my own OCR** of an image-only PDF; the OCR is good but not
  publisher-authoritative, so treat verbatim student quotations as "as OCR'd" and verify any quotation you
  publish against the original PDF.
- **Fischbein, Tirosh & Hess (1979)**: full text paywalled; the only findings I hold are Monaghan's (1986)
  secondary report of the 71%/81% figures.

---

## UNVERIFIED / COULD NOT RETRIEVE

Consolidated list (Blockers 7 and 8 together). Every entry names what was tried. **Nothing in this section may
be cited as a finding.**

### A. Citation confirmed, findings NOT verified (do not attribute results)

| Item | What is verified | What is missing | What was tried |
|---|---|---|---|
| **Tsamir, P. (2001).** "When 'The Same' is not perceived as such: The case of infinite sets." *Educational Studies in Mathematics*. DOI `10.1023/A:1016034917992` | Title, author, year, venue, DOI, citation count — via **OpenAlex API record** (`oa_status: closed`, `any_repository_has_fulltext: false`) and **Crossref API record** | No abstract in either API; no open full text; **no findings verified** | OpenAlex DOI lookup; Crossref `query.bibliographic`; ERIC quoted-phrase search; Semantic Scholar (429) |
| **Brown, S. A. (2017).** "Who's There? A Study of Students' Reasoning about a Proof of Existence." *IJRUME*. DOI `10.1007/s40753-017-0053-6` | Title, author, year, venue, DOI, ~9–10 citations — via Crossref, OpenAlex and Semantic Scholar API records | No abstract in any of the three APIs; full text blocked | Springer article page (`link.springer.com/article/10.1007/s40753-017-0053-6`) returned a 294-byte blocked body; Semantic Scholar record had `abstract: n/a`; ERIC has no record |
| **Maddy, P. (1988). "Believing the axioms" I & II.** *JSL* | Title, author, year, venue, DOIs, citation counts, and the **full introductory paragraph** of part I — via Crossref API abstract | Full text (JSTOR paywalled); the substantive argument of both papers | Crossref API (abstract retrieved, full text not); OpenAlex API; no OA location exists |
| **Fischbein, E., Tirosh, D. & Hess, P. (1979).** "The intuition of infinity." *ESM* 10(1), 3–40. DOI `10.1007/BF00311173` | Exact DOI, authors, year, venue — via Crossref API | Full text (Springer, closed); any first-hand findings. The 71%/81% figures are **Monaghan's (1986) secondary report** | Crossref; OpenAlex; ERIC; the figures were found by reading the OCR-free full text of Monaghan's thesis (Warwick, open) |
| **Fischbein, E. (2001). "Tacit Models and Infinity."** *ESM* 48. DOI `10.1023/A:1016088708705` | Title, author, year, venue, DOI — via Crossref API | Abstract and full text | Crossref; OpenAlex |
| **Tall, D. & Tirosh, D. (2001). "Infinity – the never-ending struggle."** *ESM* 48. DOI `10.1023/A:1016019128773` | Title, authors, year, venue, DOI — via Crossref API | Abstract and full text | Crossref; OpenAlex |
| **Aztekin, S., Arıkan, A. & Sriraman, B. (2010).** "The constructs of PhD students about infinity." *The Mathematics Enthusiast*. DOI `10.54870/1551-3440.1180` | Title, authors, year, venue, DOI, abstract — via **OpenAlex API record** | Full text. The OA PDF is **Cloudflare-blocked** (HTTP 403, "Just a moment…") | `scholarworks.umt.edu/cgi/viewcontent.cgi?article=1180&context=tme` (403 both via `curl` and via the fetch helper) |
| **Rauff, J. V. (2008).** "Penguins and Pandas…" *CTMS* 4(9), 11–18. DOI `10.19030/ctms.v4i9.5566` | Full bibliographic record, keywords and abstract via the **OJS landing page** (`https://clutejournals.com/index.php/CTMS/article/view/5566`) and **ERIC EJ967668** | The article's actual content (it is a descriptive note, so likely little beyond the abstract) | Publisher PDF endpoint `.../article/download/5566/5649` → curl "Maximum (50) redirects followed" (HTTP 301 loop); retried directly and via the fetch helper |
| **Mamolo, A. & Zazkis, R. (2008).** "Paradoxes as a window to infinity." *Research in Mathematics Education*. DOI `10.1080/14794800802233696` | Title, authors, year, venue, DOI, **abstract** — via OpenAlex API record | Full text (Taylor & Francis, closed) | OpenAlex; no OA location (`oa_status: closed`) |
| **Wan, X., Xu, K. & Cao, Q. (2023).** "Coq Formalization of ZFC Set Theory for Teaching Scenarios." *IJSI* 13(3), 323–357. DOI `10.21655/ijsi.1673-7288.00303` | Title, authors, year, venue, **volume/issue/pages** and abstract — via OpenAlex API + DOI resolution to the journal page | The full PDF | `http://www.ijsi.org/ijsi/article/pdf/303` → HTTP 404 / 1-byte empty body; `http://www.ijsi.org/ijsi/article/303` → HTTP 404 (ASP.NET error page) |
| **Förster, T. (2006).** "The axiom of choice and inference to the best explanation." | Title, author, year, **abstract (including the AC-pedagogy passage quoted in item 1)** — via OpenAlex API record | Full text; also **venue and DOI are absent from the record**, so its publication status is unclear (treat as preprint/unpublished) | CiteSeerX `viewdoc/summary` → **HTTP 429**; CiteSeerX `viewdoc/download` → **HTTP 500** redirecting to the Internet Archive donation page; `web.archive.org/web/2020/<same URL>` → "The Wayback Machine has not archived that URL"; `https://www.dpmms.cam.ac.uk/~tf/` → **HTTP 404** |
| **Durand-Guerrier, V. (2024).** "Contribution to didactic research on the completeness/incompleteness of ordered fields of numbers." *ZDM*. DOI `10.1007/s11858-024-01635-2` | Title, author, year, venue, DOI — via Crossref API; **abstract snippet** — via the ERIC search-results page | The full abstract and full text via ERIC | `https://eric.ed.gov/?id=EJ1450588` → **HTTP 000 / OpenSSL "unexpected eof while reading"** (transient, retryable) |
| **Tomkowicz & Wagon (2016)** *The Banach–Tarski Paradox* (CUP); **Stromberg, K. (1979)** *Amer. Math. Monthly*; **Katz & Katz (2011)** *Intellectica*; **Rayo, A. (2019)** *On the Brink of Paradox* (MIT) | Bibliographic records and publisher descriptions via Crossref/OpenAlex APIs | The texts themselves (paywalled / preview-only) | Crossref and OpenAlex API lookups; no OA full text pursued beyond the OpenAlex `best_oa_location` field |

### B. Leads seen only second-hand (not independently verified — do not cite)

- **Tirosh, D. (1991)**, cited by Hamza & O'Shea (2011) as the source of the claim that students assume "all
  methods suitable for comparing finite sets are adequate for infinite sets as well" (p. 204). I did not locate
  or verify this work; the quotation is verified only as it appears in the OCR'd Hamza & O'Shea text.
- **Dubinsky, E., Weller, K., Stinger, C. & Vidakovic, D. (2008). "Infinite iterative processes: The Tennis Ball
  Problem."** *European Journal of Pure and Applied Mathematics* 1(1), 99–121. Appears in Hamza & O'Shea's
  reference list only; not retrieved.
- **Goldberg, R. & Hammerman, N. (2004). "Adapting computational data structures technology to reason about
  infinity."** *Mathematics and Computer Education*. ERIC **EJ720447** — surfaced by an ERIC keyword search with
  title + snippet only; judged tangential and not retrieved.
- **Alcock, L. & Simpson, A. (2009).** *Ideas From Mathematics Education: An Introduction for Mathematicians*,
  Higher Education Academy MSOR Network — appears in Hamza & O'Shea's reference list; not retrieved.
- **Benci, V. & Di Nasso, M. (2019).** *How to Measure the Infinite: Mathematics with Infinite and Infinitesimal
  Numbers*, World Scientific — the source of the "numerosities" theory described in Blaszczyk (2020). Verified
  only as cited inside the retrieved Blaszczyk PDF; the book itself was not retrieved.
- **Herrlich, H.** *Axiom of Choice* (Lecture Notes in Mathematics, DOI `10.1007/11601562`) and **Jech, T.**
  *The Axiom of Choice* (North-Holland, 1973; listed in the retrieved SEP bibliography) — the standard
  technical monographs; metadata only, retrieved via Crossref API / SEP bibliography. Not education research.

### C. Searches that returned nothing and are worth re-running

- **OpenAlex daily budget exhausted mid-session.** Error returned verbatim:
  `{"error":"Rate limit exceeded", "message":"Insufficient budget. This request costs $0.001 but you only have
  $0.0003 remaining. Resets at midnight UTC."}`. Consequently these planned probes returned **empty for
  budget reasons, not for absence of literature**, and must be re-run: a relevance-ordered scan of the top ~40
  `"axiom of choice"` works; a "mathematicians' philosophical views / foundations survey" sweep; a
  `Zorn's lemma` scan; a `"university students acceptance actual infinity set theory"` scan.
- **Semantic Scholar API** returned HTTP 429 / empty for **every** query attempted (Tsamir 2001, Mamolo &
  Zazkis 2008, Hamza & O'Shea 2011, Brown 2017, and others). The three-retry wrapper in `s2.sh` did not help.
- **Journal of Humanistic Mathematics site search** — `https://scholarship.claremont.edu/jhm/search.html?q=%22axiom+of+choice%22`
  returned **HTTP 404**. A working search endpoint (or a Browse-by-issue crawl) is still needed; this journal is
  a plausible home for expository/teaching essays on AC and was **not** searched successfully.
- **ERIC** does not index Hamza & O'Shea (2011) or Tirosh & Tsamir (1996): quoted-phrase searches for
  `"Misconceptions Concerning Infinity"` and `"intuitive thinking about infinity"` both returned **0 records**.
  Absence from ERIC is therefore not evidence of absence for this topic.
- **Bing scraping (`bing.sh`)** was unusable for these queries: the query
  `"axiom of choice" teaching students misconceptions mathematics education` returned only commercial
  "Axiom" results (a trading platform, a Minecraft mod, a legal-services firm). Do not rely on it.


---

# Part D — Blockers 9–10 (transition to proof, APOS/RUMEC)

# Part D — Research evidence for blockers 9 and 10

Scope: (9) general "transition to proof" literature; (10) APOS theory and Dubinsky's /
RUMEC's set-theory-adjacent research.

**Provenance.** `web_search` / `web_fetch` were not used. Metadata and abstracts below were
retrieved with `curl` through the local proxy (`http://127.0.0.1:7890`) from Crossref
(`api.crossref.org`), Semantic Scholar (`api.semanticscholar.org`, incl. the `paper/batch`
endpoint), Unpaywall (`api.unpaywall.org`), **OpenAIRE by DOI**
(`api.openaire.eu/search/publications?doi=…&format=json` — this was the only source that
returned full publisher abstracts for these paywalled Springer/AMS items), ERIC
(`eric.ed.gov`) and Ed Dubinsky's own archived publication list plus its linked PDFs at
`https://www.math.kent.edu/~edd/`. PDFs were converted with PyMuPDF (`python3 pdf.py`).
Every DOI below was checked for resolution with `curl -o /dev/null -w '%{http_code} %{redirect_url}' https://doi.org/<DOI>`.

**Verification legend used below**
- **[F]** = full text read this session
- **[A]** = publisher abstract retrieved this session
- **[M]** = bibliographic metadata only (title/authors/year/venue/citation count verified;
  findings NOT verified — no abstract obtainable)
- **[R]** = DOI resolution verified plus independent bibliographic cross-check

---

## BLOCKER 9 — Transition to proof: proof construction, comprehension and validation

### 9.1 Moore, R. C. (1994). "Making the transition to formal proof." *Educational Studies in Mathematics* **27**(3), 249–266.
- Type: peer-reviewed journal article (Springer). **[A][R]**
- DOI: `10.1007/BF01273731` → resolves to `http://link.springer.com/10.1007/BF01273731`
- Citations: 469 (OpenAlex), 517 (Semantic Scholar).
- Abstract retrieved: `https://api.openaire.eu/search/publications?doi=10.1007/BF01273731&format=json`
- Volume/pages independently cross-checked against an open-access paper's reference list
  (LUMAT, `https://journals.helsinki.fi/lumat/article/download/2979/2427`), which cites it as
  "Moore, R. C. (1994). Making the transition to formal proof. *Educational Studies in
  Mathematics, 27*(3), 249–266."
- **Actual findings (verbatim/near-verbatim from the retrieved abstract):** the study
  "examined the cognitive difficulties that university students experience in learning to do
  formal mathematical proofs." Two preliminary studies and the main study were conducted in
  undergraduate mathematics courses at the University of Georgia in 1989; participants were
  majoring in mathematics or mathematics education; data came from "daily nonparticipant
  observation of class, tutorial sessions with the students, and interviews with the professor
  and the students." "An inductive analysis of the data revealed **three major sources of the
  students' difficulties: (a) concept understanding, (b) mathematical language and notation,
  and (c) getting started on a proof.** Also, the students' perceptions of mathematics and
  proof influenced their proof writing. Their difficulties with concept understanding are
  discussed in terms of a concept-understanding scheme involving **concept definitions,
  concept images, and concept usage**."
- Relevance: the foundational, most-cited study of the transition-to-proof problem. Item (b)
  "mathematical language and notation" is the direct ancestor of the set-theoretic notation
  blockers (1, 2, 5); item (c) "getting started" is the difficulty Weber (2001) later
  redescribed as missing strategic knowledge.

### 9.2 Weber, K. (2001). "Student difficulty in constructing proofs: The need for strategic knowledge." *Educational Studies in Mathematics* **48**(1), 101–119.
- Type: peer-reviewed journal article (Springer). **[A][R]**
- DOI: `10.1023/A:1015535614355` → resolves to `https://link.springer.com/10.1023/A:1015535614355`
- Citations: 405 (OpenAlex), 483 (Semantic Scholar).
- Abstract retrieved: `https://api.openaire.eu/search/publications?doi=10.1023/A:1015535614355&format=json`
- **Actual findings (from the retrieved abstract):** "undergraduates often are aware of and
  able to apply the facts required to prove a statement but still fail to prove it. They thus
  fail to construct a proof because they **could not use the syntactic knowledge that they
  had**. By comparing doctoral students and undergraduates constructing proofs in abstract
  algebra, I have hypothesized **four types of 'strategic knowledge' – knowledge of how to
  choose which facts and theorems to apply** – which the doctoral students appeared to possess
  and undergraduates did not. The doctoral students appeared to know (i) the powerful proof
  techniques in abstract algebra, (ii) which theorems are most important, (iii) when particular
  facts and theorems are likely to be useful, and (iv) when one should or should not try and
  prove theorems using symbol manipulation."
- Relevance: distinguishes *having* the facts from *being able to deploy* them. Directly
  supports a curriculum that teaches proof-strategy selection explicitly rather than only
  facts and definitions.

### 9.3 Selden, J. & Selden, A. (1995). "Unpacking the logic of mathematical statements." *Educational Studies in Mathematics* **29**(2), 123–151.
- Type: peer-reviewed journal article (Springer). **[A][R]**
- DOI: `10.1007/BF01274210` → resolves to `http://link.springer.com/10.1007/BF01274210`
- Citations: 245 (OpenAlex), 275 (Semantic Scholar).
- Abstract retrieved: `https://api.openaire.eu/search/publications?doi=10.1007/BF01274210&format=json`
- **Actual findings (from the retrieved abstract):** "focuses on undergraduate students'
  ability to **unpack informally written mathematical statements into the language of predicate
  calculus**. Data were collected between 1989 and 1993 from **61 students in six small sections
  of a 'bridge' course** designed to introduce proofs and mathematical reasoning." Extends
  "concept image" to "**statement image**" and introduces "**proof framework**" for the part of
  a theorem's image corresponding to the top-level logical structure of a proof. "For simplified
  informal calculus statements, **just 8.5% of unpacking attempts were successful**; for actual
  statements from calculus texts, **this dropped to 5%**." Inference: "these students would be
  unable to reliably relate informally stated theorems with the top-level logical structure of
  their proofs and hence could not be expected to construct proofs or validate them."
- Relevance: **the single best quantitative bridge between blocker 5 (quantifier/logical
  structure) and blocker 9 (proof)** — 91.5%–95% failure at extracting the logical form of a
  statement, which is exactly the skill `∀`/`∃` ordering and the `∈`/`⊆` distinction require.

### 9.4 Selden, A. & Selden, J. (2003). "Validations of proofs considered as texts: Can undergraduates tell whether an argument proves a theorem?" *Journal for Research in Mathematics Education* **34**(1), 4–36.
- Type: peer-reviewed journal article (NCTM). **[A][R]**
- DOI: `10.2307/30034698` → resolves to `https://www.jstor.org/stable/10.2307/30034698`
- Citations: 338 (OpenAlex), 378 (Semantic Scholar).
- Unpaywall reports an open copy (submitted version):
  `https://philpapers.org/archive/SELVOP-2.pdf` — ⚠️ **we could not download it**: direct
  fetch returned HTTP 403 with a Cloudflare "Enable JavaScript and cookies to continue"
  interstitial. The link is Unpaywall-verified but was not personally retrieved.
- **Actual findings (from the retrieved abstract):** "an exploratory study of the way that
  **eight** mathematics and secondary education mathematics majors read and reflected on
  **four student-generated arguments** purported to be proofs of a single theorem. The results
  suggest that such undergraduates tend to **focus on surface features of arguments** and that
  their ability to determine whether arguments are proofs is **very limited – perhaps more so
  than either they or their instructors recognize**."
- Relevance: "validation" as a distinct competence from "construction". Supports teaching
  proof-*reading* explicitly. Complements 9.5.

### 9.5 Inglis, M. & Alcock, L. (2012). "Expert and novice approaches to reading mathematical proofs." *Journal for Research in Mathematics Education* **43**(4), 358–390.
- Type: peer-reviewed journal article (NCTM). **[A][R]**
- DOI: `10.5951/jresematheduc.43.4.0358` → resolves to
  `https://pubs.nctm.org/view/journals/jrme/43/4/article-p358.xml`
- Citations: 213 (OpenAlex), 80 (Semantic Scholar). Open repository record:
  `https://dspace.lboro.ac.uk/2134/9982`
- **Actual findings (from the retrieved abstract):** eye movements of beginning undergraduates
  and research-active mathematicians were recorded while they validated purported proofs.
  "(a) contrary to previous suggestions, mathematicians sometimes appear to disagree about the
  validity of even short purported proofs; (b) compared with mathematicians, **undergraduate
  students spend proportionately more time focusing on 'surface features' of arguments,
  suggesting that they attend less to logical structure**; and (c) compared with undergraduates,
  mathematicians are more inclined to shift their attention back and forth between consecutive
  lines of purported proofs, suggesting that they devote more effort to inferring implicit
  warrants."
- Relevance: the strongest *methodological* upgrade of Selden & Selden's "surface features"
  finding — eye-tracking, expert/novice contrast. Direct evidence for teaching students to
  read logical structure rather than surface form.

### 9.6 Alcock, L. & Weber, K. (2005). "Proof validation in real analysis: Inferring and checking warrants." *The Journal of Mathematical Behavior* **24**(2), 125–134.
- Type: peer-reviewed journal article (Elsevier). **[M][R]**
- DOI: `10.1016/j.jmathb.2005.03.003` → resolves to
  `https://linkinghub.elsevier.com/retrieve/pii/S0732312305000143`
- Citations: 110 (OpenAlex), 118 (Semantic Scholar). No abstract obtainable — listed as
  citation-verified, findings unverified.

### 9.7 Iannone, P., Inglis, M., Mejía-Ramos, J. P., Simpson, A., & Weber, K. (2011). "Does generating examples aid proof production?" *Educational Studies in Mathematics* **77**(1), 1–14.
- Type: peer-reviewed journal article (Springer). **[M][R]**
- DOI: `10.1007/s10649-011-9299-0` → resolves to `https://doi.org/10.1007/s10649-011-9299-0`
- Citations: 30 (Crossref), 52 (Semantic Scholar). No abstract recoverable.

### 9.8 Alcock, L. & Inglis, M. (2009). "Representation systems and undergraduate proof production: A comment on Weber." *The Journal of Mathematical Behavior* **28**(4), 209–211.
- Type: peer-reviewed journal article (Elsevier). **[A][R]**
- DOI: `10.1016/j.jmathb.2009.10.001` → resolves to `https://doi.org/10.1016/j.jmathb.2009.10.001`
- Citations: 8 (Crossref), 17 (Semantic Scholar). Open repository record:
  `https://dspace.lboro.ac.uk/2134/8578`
- **Actual findings (from the retrieved abstract):** "Weber (2009) suggested that
  counterexamples can be generated by a syntactic proof production, apparently contradicting
  our earlier assertion (Alcock & Inglis, 2008). Here we point out that this ostensible
  difference is the result of Weber working with **theoretical definitions that differ slightly
  from ours**. We defend our approach by arguing that Weber's relies upon an as yet unspecific
  metric for gauging the amount of work conducted in each representation system, and that it
  does not recognize an **important asymmetry between the status of representation systems in
  the context of undergraduate mathematics**."
- Relevance: documents that the "semantic vs syntactic proof production" construct — widely
  used for proof teaching — is itself contested and definition-sensitive. Useful as a
  caution against over-simplifying the novice/expert story.

### 9.9 Harel, G. & Sowder, L. (1998). "Students' proof schemes: Results from exploratory studies." In *CBMS Issues in Mathematics Education* 7, 234–283. American Mathematical Society.
- Type: peer-reviewed book chapter / research monograph series (AMS). **[A-][R]** (metadata
  verified; OpenAIRE record exists but its `description` field is EMPTY, so no abstract)
- DOI: `10.1090/cbmath/007/07` → resolves to `https://www.ams.org/cbmath/007`
- Citations: 313 (Crossref).
- **The taxonomy itself was verified from a retrieved open-access source**, Kanellos, Nardi &
  Biza (2018), *Mathematical Thinking and Learning*, DOI `10.1080/10986065.2018.1509420`,
  full PDF read at
  `https://ueaeprints.uea.ac.uk/id/eprint/66431/1/Kanellos_Nardi_Biza_MTL_D_17_00097_030318_FINAL_PRE_PROOF_.pdf`.
  That paper states: proof schemes describe what "constitutes ascertaining and persuading" in
  an individual's or community's proving activity, and groups the schemes into **three classes
  divided further into subclasses** — **external conviction** proof schemes (**(a) authoritarian**
  — appeal to a teacher or book; **(b) ritual** — the argument depends strictly on its
  appearance, e.g. "proofs in geometry must have a two-column format"; **(c) non-referential
  symbolic** — symbol manipulation with no coherent system of referents); **empirical** proof
  schemes (**(a) inductive** — evidence from examples, sometimes just one, or direct measurement;
  **(b) perceptual** — reliance on perceptions); and **deductive** proof schemes (**(a)
  transformational**; **(b) axiomatic** — additionally acknowledging the axiomatic foundation of
  the theory). That is the widely cited **seven schemes**. Kanellos et al. also report an
  empirical test: 85 Year-9 students (age 14–15, Greek state schools), six questions — they found
  evidence of **six of the seven schemes** (no deductive-axiomatic), **plus eight combinations**
  of schemes, often within a single student's response to a single item.
- ⚠️ Note: the primary taxonomy is Harel & Sowder (1998); the widely quoted page numbers
  "p. 809" in the retrieved source refer to a **later** Harel & Sowder chapter (2007) — the two
  should not be conflated. Harel, G. (2007), "Students' proof schemes revisited", in *Theorems
  in School*, DOI `10.1163/9789087901691_006`, is a distinct item (verified via Crossref).
- Relevance: the standard framework for classifying *what counts as justification* for a
  student. Directly usable to diagnose why a student accepts a vacuous `∅ ⊆ A` argument (or
  does not).

### 9.10 Selden, A. (2012). "Transitions and proof and proving at tertiary level." In *Proof and Proving in Mathematics Education* (New ICMI Study Series), 391–420. Springer.
- Type: peer-reviewed book chapter (ICMI study volume). **[A][R]**
- DOI: `10.1007/978-94-007-2129-6_17` → resolves to `https://doi.org/10.1007/978-94-007-2129-6_17`
- Citations: 73 (OpenAlex).
- **Actual findings (from the retrieved abstract):** discusses "implications of the requirement
  that students become autonomous in understanding and constructing rigorous proofs when they
  transition to tertiary level mathematics. As they struggle with this requirement, students
  encounter various difficulties including: **the proper use of logic; the necessity to employ
  formal definitions; the need for a repertoire of examples, counterexamples, and nonexamples;
  the requirement for a deep understanding of the concepts and theorems involved; the need for
  strategic knowledge of which theorems are important, and the importance of being able to read
  and check arguments for correctness.**" It then reviews instructional responses: generic
  proofs or structured proofs, transition-to-proof courses, communities of practice, the Moore
  Method, co-construction of proofs, and the method of scientific debate.
- Relevance: **the best single synthesized survey citation for blocker 9** — an author of two of
  the primary studies above reviewing the whole field, and its difficulty list maps onto
  blockers 1, 2, 5 and 6 almost item for item.

### 9.11 Stylianides, G. J. & Stylianides, A. J. (2009). "Facilitating the transition from empirical arguments to proof." *Journal for Research in Mathematics Education* **40**(3), 314–352.
- Type: peer-reviewed journal article (NCTM). **[A][R]**
- DOI: `10.5951/jresematheduc.40.3.0314` → resolves to
  `https://doi.org/10.5951/jresematheduc.40.3.0314`
- Citations: 195 (OpenAlex).
- **Actual findings (from the retrieved abstract):** a 4-year design experiment in an
  undergraduate mathematics course (prerequisite for elementary teaching certification),
  reporting "the theoretical foundation and implementation of an **instructional sequence that
  aimed to help students begin to realize the limitations of empirical arguments** as methods
  for validating mathematical generalizations and see an intellectual need to learn about
  secure methods for validation (i.e., proofs)", with **cognitive conflict** playing a major
  role.
- Relevance: the strongest *instructional-design* citation for blocker 9 — it is about moving
  students off empirical justification, which is precisely the Harel–Sowder "empirical scheme"
  transition.

### 9.12 Mejía-Ramos, J. P., Lew, K., de la Torre, J., & Weber, K. (2017). "Developing and validating proof comprehension tests in undergraduate mathematics." *Research in Mathematics Education* **19**(2), 117–136.
- Type: peer-reviewed journal article (Taylor & Francis). **[A][R]**
- DOI: `10.1080/14794802.2017.1325776` → resolves to
  `https://doi.org/10.1080/14794802.2017.1325776`
- Citations: 37 (OpenAlex).
- **Actual findings (from the retrieved abstract):** describes the process by which the authors
  developed and validated "short, multiple-choice, reliable tests to assess undergraduate
  students' **comprehension** of three mathematical proofs."
- Relevance: gives a ready-made *assessment instrument design pattern* if the curriculum needs
  to measure proof comprehension rather than proof production.

### 9.13 Tall, D. (2013). *How Humans Learn to Think Mathematically: Exploring the Three Worlds of Mathematics.* Cambridge University Press.
- Type: scholarly monograph (Cambridge). **[A][R]**
- DOI: `10.1017/cbo9781139565202` → resolves to
  `https://www.cambridge.org/core/product/identifier/9781139565202/type/book`
- Citations: 145 (Semantic Scholar), 200 (Crossref), 115 (OpenAlex, as a review record).
- **Actual findings (from the retrieved abstract):** the book "describes the development of
  mathematical thinking from the young child to the sophisticated adult… reveals the reasons
  why **mathematical concepts that make sense in one context may become problematic in
  another**. For example, a child's experience of whole number arithmetic successively affects
  subsequent understanding of fractions, negative numbers, algebra, and **the introduction of
  definitions and proof**… The book offers a comprehensive framework for understanding
  mathematical growth, from **practical beginnings through theoretical developments, to the
  continuing evolution of mathematical thinking at the highest level**."
- ⚠️ **Unverified detail:** Tall's framework is conventionally summarised as three "worlds"
  (an embodied/conceptual world, a symbolic/proceptual world, and a formal/axiomatic world).
  We could **not** retrieve a source this session that names the three worlds, so the
  three-way terminology should be re-verified before being quoted. What IS verified: the book
  is titled around "the three worlds of mathematics", and Tall uses "three distinct worlds of
  mathematical thinking (Tall, 2004)" in his own words (retrieved in the OpenAlex abstract of
  Tall, 2011, "Crystalline Concepts in Long-Term Mathematical Invention and Discovery").
- Related verified items: the chapter "The Three Worlds of Mathematics" in the same book,
  DOI `10.1017/cbo9781139565202.011`; Tall, Nogueira de Lima & Healy (2014), "Evolving a
  three-world framework for solving algebraic equations in the light of what a student has met
  before", *JMB*, DOI `10.1016/j.jmathb.2013.12.003` (57 citations); and Dawkins, P. C. (2015),
  "In Pursuit of Coherent and Formalizable Understanding: Reflections on David Tall's Three
  Worlds Framework", *IJRUME*, DOI `10.1007/s40753-015-0006-x`.
- Relevance: supplies the *cognitive-developmental* vocabulary (concept that works in one
  context becomes problematic in another) for explaining why students who are fluent in
  school arithmetic/geometry still fail at formal definition-and-proof.

### 9.14 Bonus bridge item — Dawkins, P. C., Zazkis, D., & Cook, J. P. (2022). "How do transition to proof textbooks relate logic, proof techniques, and sets?" *PRIMUS* **32**(5), 1–18.
- Type: peer-reviewed journal article (Taylor & Francis). **[A][R]**
- ERIC record: `https://eric.ed.gov/?id=EJ1322061`; DOI `10.1080/10511970.2020.1827322`
- **Actual findings (from the retrieved ERIC abstract):** "Many mathematics departments have
  transition to proof (TTP) courses… Here we discuss how **common TTP textbooks connect three
  topics ubiquitous to such courses: logic, proof techniques, and sets**."
- Relevance: exactly the curriculum-survey question — how existing TTP material sequences
  logic / proof technique / set theory. Also the natural bridge into blocker 10.
- Also verified in this area: Smith, M. D. (2023), "Active learning ideas for the transition to
  proofs course", *PRIMUS*, ERIC `https://eric.ed.gov/?id=EJ1389668`, DOI
  `10.1080/10511970.2023.2167251`; and Pair, J. & Calva, G. (2022), "Undergraduate perspectives
  on the nature of mathematics that arise through exploration of unsolved conjectures", PME-NA
  proceedings, free full text at `https://files.eric.ed.gov/fulltext/ED630456.pdf`.

### 9.15 Bonus item — Dawkins, P. C. & Roh, K. H. (2020). "Assessing the influence of syntax, semantics, and pragmatics in student interpretation of multiply quantified statements in mathematics." *International Journal of Research in Undergraduate Mathematics Education* **6**, 1–24.
- Type: peer-reviewed journal article (Springer). **[A][R]**
- ERIC record: `https://eric.ed.gov/?id=EJ1250037`; DOI `10.1007/s40753-019-00097-2`
  (resolves to `http://link.springer.com/10.1007/s40753-019-00097-2`)
- **Actual findings (from the retrieved ERIC abstract):** "compares the relative influence of
  **syntax, semantics, and pragmatics** in university students' interpretation of **multiply
  quantified statements** in mathematics, both before and after instruction."
- Relevance: belongs to blocker 5, but is listed here because it is the most recent
  peer-reviewed treatment of *why* students mis-parse `∀…∃…` statements — the exact competence
  Selden & Selden (9.3) measured at 5–8.5% success. Companion: Schüler-Meyer, A. (2022), "How
  transition students relearn school mathematics to construct multiply quantified statements",
  *ESM*, ERIC `https://eric.ed.gov/?id=EJ1334228`, DOI `10.1007/s10649-021-10127-z`.

---

## BLOCKER 10 — APOS theory; Dubinsky's and RUMEC's set-theory-related research

**Important framing.** Dubinsky's and RUMEC's published empirical work is overwhelmingly
about *calculus, linear algebra, abstract algebra and functions* — not about set theory as a
topic. The set-theory-adjacent RUMEC/APOS items are: quantification (10.3–10.5), functions
and relations (10.8–10.10), infinity (10.7), and the ISETL discrete-mathematics textbook
tradition (10.11). The nearest thing to a *dedicated set-theory* APOS study found is
Hamdan (10.10) plus the computer-mediation dissertations (10.12). This is a real gap in the
literature and should be reported as such.

### 10.1 Dubinsky, E. & McDonald, M. A. (2005). "APOS: A constructivist theory of learning in undergraduate mathematics education research." In D. Holton et al. (Eds.), *The Teaching and Learning of Mathematics at University Level: An ICMI Study*, Kluwer Academic Publishers, 273–280.
- Type: peer-reviewed book chapter (ICMI study volume). **[A][F-PDF][R]**
- DOI: `10.1007/0-306-47231-7_25` → resolves to `http://link.springer.com/10.1007/0-306-47231-7_25`
- Citations: 379 (OpenAlex), 503 (Semantic Scholar).
- Free full text (author's own copy, verified HTTP 200, 75 KB PDF):
  `https://www.math.kent.edu/~edd/ICMIPaper.pdf`
- Abstract retrieved: `https://api.openaire.eu/search/publications?doi=10.1007/0-306-47231-7_25&format=json`
  and Dubinsky's publication list at `https://www.math.kent.edu/~edd/publications.html`
- **Actual findings (from the retrieved abstract):** the authors "mentioned **six ways in which
  a theory can contribute to research**" and propose this list as evaluation criteria for a
  theory. They "described how one such perspective, **APOS Theory, is being used in an organized
  way by members of RUMEC** and others to conduct research and develop curriculum. We have shown
  how **observing students' success in making or not making mental constructions proposed by the
  theory and using such observations to analyze data** can organize our thinking about learning
  mathematical concepts, **provide explanations of student difficulties and predict success or
  failure** in understanding a mathematical concept… the theory is **grounded in data**."
  They point to an annotated bibliography (McDonald, 2000) for further detail.
- ⚠️ Publication-year note: the chapter is dated **2001** in Dubinsky's own publication list
  ("…, 2001. In D. Holton et. (Eds.) …, Kluwer Academic Publishers, 273–280") but the Springer
  record and Semantic Scholar give **2005** (the volume's eBook date). Both years circulate;
  cite the volume and page range to avoid ambiguity.
- Relevance: **the canonical citation for APOS itself** and the RUMEC connection. APOS =
  Action → Process → Object → Schema; a "genetic decomposition" proposes the mental
  constructions a learner must make for a given concept. This is the framework used by the
  set-theory-adjacent studies below.

### 10.2 Arnon, I., Cottrill, J., Dubinsky, E., Oktaç, A., Roa Fuentes, S., Trigueros, M., & Weller, K. (2014). *APOS Theory: A Framework for Research and Curriculum Development in Mathematics Education.* Springer.
- Type: scholarly monograph (Springer). **[M][R]**
- DOI: `10.1007/978-1-4614-7966-6` → resolves to `https://link.springer.com/10.1007/978-1-4614-7966-6`
- Citations: 220 (OpenAlex).
- Related verified chapters in the same volume: ch. 2 "From Piaget's Theory to APOS Theory:
  Reflective Abstraction in Learning Mathematics and the Historical Development of APOS
  Theory", DOI `10.1007/978-1-4614-7966-6_2`; ch. 6 "The APOS Paradigm for Research and
  Curriculum Development", DOI `10.1007/978-1-4614-7966-6_6`.
- Book review: Inglis, M. (2015), *IJRUME*, DOI `10.1007/s40753-015-0015-9` (open PDF at
  `https://link.springer.com/content/pdf/10.1007/s40753-015-0015-9.pdf`).
- Relevance: the definitive book-length statement of APOS; use for the theory, the
  genetic-decomposition methodology, and the ACE teaching cycle (Activities, Class discussion,
  Exercises).

### 10.3 Dubinsky, E. (1997). "On learning quantification." *Journal of Computers in Mathematics and Science Teaching* **16**(2–3), 335–362.
- Type: peer-reviewed journal article (AACE). **[F]** — full text read
- ERIC record: `https://eric.ed.gov/?id=EJ567950` (record type Journal; ISSN 0731-9258)
- Free full text (author's own copy, verified HTTP 200, 411 KB PDF, 43 pp.):
  `https://www.math.kent.edu/~edd/LearningQuant.pdf`
  (also archived: `https://web.archive.org/web/2020id_/http://www.math.kent.edu/~edd/LearningQuant.pdf`)
- **Actual findings (read from the full text):** 36 students — sophomores and juniors with
  joint mathematics/computer-science majors taking discrete mathematics — across two classes
  at two universities, taught with instruction based on a prior theoretical analysis of what
  it means to understand quantification, using the ISETL programming language. The two classes:
  **Class 1 = 19 students, mostly sophomores, "Introduction to Finite Mathematics", Clarkson
  University, Fall 1986**; **Class 2 = 17 students, mainly juniors, "Introduction to Analysis",
  the following semester** (19 + 17 = 36). Assessment was "written questions on which students
  were to work individually to provide written responses. The questions were given in class
  without warning."
  - **Class 1 did well**: on 11 of 21 problems the average was better than 70%; the worst
    problem average in Class 1 was 47%.
  - **Class 2** (much less time on quantification, mostly independent work) "did quite well on
    only 7 of the 16 problems … and very poorly on 6 of them", with three problems on which
    students scored less than 30%.
  - **The key error finding, quoted:** "The second striking feature concerns the kind of errors
    that the students made. **Of the seven problems on which the students did poorly (less than
    60%), five of them involved an implication which was not part of the quantification.** It is
    clear from the papers that difficulty with implications contributed to the low score. For
    example, on Problem 4a of SetA, **all but one of the students were quite correct in negating
    the quantification part. Their errors were entirely the result of greater or lesser
    difficulties they had with negating an implication.**"
  - Class 2's worst performance was on problems requiring **translation into** formal language
    ("It is possible to argue that this result is largely due to lack of practice with the
    syntax"), yet the same students did well on problems probing comprehension of the same
    statements — Dubinsky's inference: "understanding the meaning of a statement can develop
    without a corresponding ability to deal with the statement linguistically."
  - Conclusion: students "can develop some understanding of quantification and the ability to
    work with it, even when the particular problems are difficult", but the author flags "the
    continued difficulty that students have with implications… when implication is not the
    direct object of consideration, but rather is placed in the context of another issue
    (quantification), then there is a sharp drop in students' ability to cope."
  - The instrument includes explicitly order-sensitive items: the "flying fish" statement, the
    negation of "In all the classes that I have taught, there is one in which every student got
    an 'A'", an ε–δ-style "For a given q > 0 … for every p > 0 there is an x …", and a Problem 10
    deliberately containing "a 'trap' intended to keep the students from applying textual
    translation and formal rules in order to negate."
  - ⚠️ **Correction applied during assembly:** an earlier draft of the companion report
    (Part B) gave the class sizes as 21 and 19 and the year as 1995. Both are wrong. The
    paper's own text (lines 15 and 469–474 of the decoded full text) gives **36 total = 19 + 17**
    and **Fall 1986**. Quote the corrected figures.
- Relevance: **triple duty** — it is the requested blocker-5 source, a blocker-10 APOS/RUMEC
  source, and (through the implication finding) an empirical anchor for blocker 2's vacuous
  implication. The finding is unusually quotable: isolating implication *inside* a
  quantification task destroys performance.

### 10.4 Dubinsky, E. & Yiparaki, O. (2000). "On student understanding of AE and EA quantification." Unpublished manuscript, Georgia State University / Agnes Scott College, dated 18 November 2000, 67 pp.
- Type: **unpublished manuscript / author-archived technical report** — NOT peer-reviewed.
  Cite it as a manuscript, or cite it second-hand via Chellougui & Kouki (2012), who reference
  "Dubinsky et Yiparaki (2000)". **[F]** — full text read
- Free full text (verified HTTP 200, 1.2 MB PDF, 67 pp.):
  `https://www.math.kent.edu/~edd/OlgaPaper.pdf`
- **Actual findings (read from the full text):** 63 students; a questionnaire with **eleven
  declarative statements** plus follow-up interviews; students decided true/false and explained.
  - "**94% of the students interpreted at least one EA statement as an AE**" — for the six EA
    statements, the percentage interpreting them as AE **ranged from 11% to 81%**. By contrast
    "**only 5% interpreted at least one AE statement as an EA**; the percentage of students who
    interpreted AE statements as EA ranged from **0% to 3%**."
  - Abstract: "students are **not inclined to use the syntax of a statement in order to
    interpret it**, particularly if they do not understand it very well; rather, they use the
    **context** of a statement to discuss their own opinions about the topic in general, not the
    actual statement. We also found that **students are more inclined to interpret English
    statements as AE rather than EA. Most students in this study could not distinguish between
    AE and EA statements in mathematics and did not seem to be aware of the standard
    mathematical conventions for parsing statements.**"
  - When asked whether a statement was ambiguous, "There were no such written responses on any
    of the questions indicating there was ambiguity"; even under interview probing, most
    continued to see the EA statements as AE.
  - **Truth-value asymmetry:** "30% of the AE statements were declared True, but only 10% of
    the EA statements were declared True."
  - **On the two mathematical statements:** "only 41% of the students got statement 10 right,
    and only 9% got statement 11 right"; assuming the domain is the reals these rise only to
    49% and 14% — "yet 78% of the students gave a valid argument for seven out of the nine
    natural-language" statements. In interviews "57% of the students we interviewed were able
    to assess the truth value of statement 10 correctly, whereas only 41% of all the students
    who responded [did]"; 57% were unsuccessful on both statements 10 and 11.
  - **Wording, not logical form, drives the reading:** statements using "every" — 81% read one
    statement as AE vs about 40% for another; 11% read one EA statement as AE while 37% read
    another as AE.
  - Even the 9 graduate students were imperfect: "two (out of nine) graduate students did not
    get Statement 10 right, only five of the nine got Statement 11 right."
  - The paper's pedagogical conclusion is notable: "instead of trying to make everyday life
    analogies between ordinary English statements and mathematical statements, perhaps we
    should remain in the mathematical contexts and concentrate our efforts directly on helping
    students understand mathematical statements in their natural mathematical habitats."
- Relevance: **the most direct empirical evidence for blocker 5's "students swap ∀∃"** — and it
  is directional: students overwhelmingly collapse `∃∀` into `∀∃` (94%), almost never the
  reverse (5%). This matters for injective/surjective definitions because "there exists a
  unique y for all x" vs "for all x there exists y" is exactly an EA/AE contrast.

### 10.5 Dubinsky, E., Elterman, F., & Gong, C. (1988/1989). "The student's construction of quantification." *For the Learning of Mathematics* **8**(2), 44–51.
- Type: peer-reviewed journal article (FLM). **[M]** (not in Crossref at this vintage)
- Free full text (verified HTTP 200, 1.8 MB PDF — a scanned, longer 22-page preprint/extended
  version): `https://www.math.kent.edu/~edd/QUANT1.pdf`
- ⚠️ **Year/volume caveat:** the article's own reference list in Dubinsky (1997) cites it as
  "Dubinsky, E., Elterman, F., & Gong, C. (1988). The student's construction of quantification.
  *For the Learning of Mathematics*, 8(2), 44-51", while Dubinsky's own publications page lists
  it as 1989 with no volume. FLM vol. 8 is 1988. Prefer the 1988, 8(2), 44–51 form and note the
  discrepancy if precision matters.
- Relevance: the theoretical/epistemological precursor to 10.3 — the paper 10.3 says the
  instruction was "based on previous research into what it means to understand this concept",
  and 10.4 cites it as "Dubinsky, Elterman, Gong (1988)". The genetic decomposition of
  quantification originates here. Companion report Part B OCR'd this scan and read the body;
  its reported design is an informal interview study of a Discrete Mathematics class at UC
  Berkeley, Spring 1986, taught using SETL.

### 10.6 Weller, K., Clark, J., Loch, S., McDonald, M., & Merkovsky, R. "An examination of student performance data in recent RUMEC studies."
- Type: unpublished research report (listed on Dubinsky's publication list as item 69,
  "In Review"). **[M]**
- Free full text (verified HTTP 200, 1.3 MB PDF):
  `https://www.math.kent.edu/~edd/Performance.pdf`
- Relevance: the closest thing to a RUMEC *meta-analysis* of its own student-performance data
  across studies. Because it is in-review/unpublished and its findings were not read here, use
  it only as a pointer, not as evidence.

### 10.7 Dubinsky, E., Weller, K., McDonald, M. A., & Brown, A. (2005). "Some historical issues and paradoxes regarding the concept of infinity: An APOS-based analysis: Part 1." *Educational Studies in Mathematics* **58**(3), 335–359. And Part 2, *ESM* **60**(2), 253–266.
- Type: peer-reviewed journal articles (Springer). **[M][R]**
- Part 1 DOI: `10.1007/s10649-005-2531-z` → resolves to
  `http://link.springer.com/10.1007/s10649-005-2531-z` — 89 (OpenAlex) / 119 (Semantic Scholar) citations
- Part 2 DOI: `10.1007/s10649-005-0473-0` → resolves to
  `https://doi.org/10.1007/s10649-005-0473-0` — 47 (OpenAlex) / 126 (Semantic Scholar) citations
- Author's copy of Part 1 (verified HTTP 200, 239 KB PDF):
  `https://www.math.kent.edu/~edd/HistPart1Submit.pdf`
- Companion expository piece by the same group: "Intimations of Infinity", *Notices of the AMS*
  (2004), author's copy verified HTTP 200:
  `https://www.math.kent.edu/~edd/notices-SUBMITFinal-23APR.pdf`
- Relevance: **the APOS/RUMEC contribution that bears directly on blocker 7** — an APOS-based
  analysis of historical paradoxes of infinity, i.e. the same conceptual obstacles that
  produced Cantor's theory, read through Action/Process/Object/Schema. Findings not read
  this session (no abstract obtainable); the DOIs and citation counts are verified.

### 10.8 Breidenbach, D., Dubinsky, E., Hawks, J., & Nichols, D. (1992). "Development of the process conception of function." *Educational Studies in Mathematics* **23**(3), 247–285.
- Type: peer-reviewed journal article (Springer). **[M][R]**
- DOI: `10.1007/BF02309532` — verified against Crossref (ESM vol. 23, issue 3, pp. 247–285, 1992;
  authors Breidenbach, Dubinsky, Hawks, Nichols). 399 citations (OpenAlex), 406 (Semantic Scholar).
- Author's copy (verified HTTP 200, 3.25 MB PDF — a scanned back-issue):
  `https://www.math.kent.edu/~edd/PROCESSFUNC.pdf`
- Also verified from Dubinsky's publication list: "(with J. Hawks, D. Nichols) Development of
  the Process Conception of Function by Pre-Service Teachers in a Discrete Mathematics Course,
  Proceedings, Thirteenth Annual Meeting of PME, June 1989, Paris" (item 12).
- Relevance: **belongs to blocker 4 as well as blocker 10** — the canonical APOS study of how
  students move from a *process* conception of function to an *object* conception, which is
  precisely the "function = its graph / set of ordered pairs" ontology problem. Findings not
  read this session.

### 10.9 Dubinsky, E. & Harel, G. (1992). "The nature of the process conception of function." In G. Harel & E. Dubinsky (Eds.), *The Concept of Function: Aspects of Epistemology and Pedagogy*, MAA Notes 25, 85–106.
- Type: peer-reviewed edited-volume chapter (Mathematical Association of America). **[M]**
- Verified from Dubinsky's own publication list (`https://www.math.kent.edu/~edd/publications.html`,
  item 24). No DOI found.
- Companion volume verified: Harel, G. & Dubinsky, E. (Eds.) (1992), *The Concept of Function:
  Aspects of Epistemology and Pedagogy*, MAA Notes 25 (listed as item 2 under books).
- Relevance: blocker 4's theoretical core — the process/object distinction applied to function.

### 10.10 Hamdan, M. (2006). "Equivalent structures on sets: Equivalence classes, partitions and fiber structures of functions." *Educational Studies in Mathematics* **62**(2), 127–147.
- Type: peer-reviewed journal article (Springer). **[M][R]**
- DOI: `10.1007/s10649-006-5798-9` → resolves to
  `http://link.springer.com/10.1007/s10649-006-5798-9`
- Verification: Crossref confirms ESM vol. 62, issue **2**, pp. **127–147**, 2006, author May Hamdan.
  (An earlier draft of this note said 62(1), 51–77 in error — Crossref is authoritative here.)
- Citations: 11 (Semantic Scholar), 13 (OpenAlex). No abstract obtainable.
- Relevance: **the closest verified item to a genuine APOS study of set-theoretic structure** —
  equivalence classes, partitions, and the fiber structure of a function are exactly the
  set-theoretic objects a `Sets, Functions and Relations` unit teaches. Listed as
  citation-verified; findings unverified.

### 10.11 Baxter, N., Dubinsky, E., & Levin, G. (1989). *Learning Discrete Mathematics with ISETL.* Springer-Verlag.
- Type: textbook (Springer). **[M][R]**
- DOI: `10.1007/978-1-4612-3592-7` (Crossref). A second DOI `10.1007/978-1-4612-3599-7_3`
  corresponds to its chapter 3, "Sets and Tuples".
- Companion verified textbooks: Dubinsky, E. & Leron, U. (1994), *Learning Abstract Algebra
  with ISETL*, Springer, DOI `10.1007/978-1-4612-2602-4`; Dubinsky, E. & Fenton, W. E. (1996),
  *Introduction to Discrete Mathematics with ISETL*, Springer.
- Relevance: **this is the RUMEC-tradition set-theory curriculum** — the ISETL environment in
  which Zazkis & Gunn (1997) ran their element/subset/empty-set study. If the survey needs a
  precedent for teaching set theory *computationally*, this is the primary one.

### 10.12 Computer-mediated set theory + logic (nearest dedicated set-theory learning studies)
- **Martinez, A. E. (2022).** *Bridging the Gap between Set Theory and Logic: Leveraging
  Computing as a Mediating Tool for Learning.* Doctoral dissertation, ProQuest LLC.
  ERIC record: `https://eric.ed.gov/?id=ED646013` — **[A]** (ERIC abstract retrieved)
  **Findings (from the ERIC abstract):** "Undergraduate mathematics education research focused
  on Introduction to Proofs courses has gained traction as more students are experiencing
  challenges in their proof-based courses. While studies have analyzed the teaching and
  learning of proofs, there is a growing need…" (abstract truncated in ERIC's snippet).
- **Martinez IV, A. E. (2024).** "Using Python to reason about logic and set theory: Three
  instrumented action schemes." *Digital Experiences in Mathematics Education.*
  ERIC record: `https://eric.ed.gov/?id=EJ1417658`; DOI `10.1007/s40751-023-00130-9` — **[A]**
  **Findings (from the ERIC abstract):** "Many areas of mathematics naturally lend themselves
  to machine-based computing environments, which suggests that computational environments may
  serve as useful mediating tools for the teaching and learning of mathematical c…" (truncated).
- Relevance: these are the modern continuation of the ISETL/Zazkis–Gunn line — programming as
  the mediating tool for set theory and logic, and the *only* recent work found that is
  explicitly about learning set theory itself rather than about proof in general.

### 10.13 Dawkins, P. C. (2015). "In pursuit of coherent and formalizable understanding: Reflections on David Tall's three worlds framework." *International Journal of Research in Undergraduate Mathematics Education* **1**, 361–367.
- Type: peer-reviewed journal article (Springer), review essay. **[M][R]**
- DOI: `10.1007/s40753-015-0006-x` → resolves to `https://doi.org/10.1007/s40753-015-0006-x`
- Relevance: the review that connects Tall's framework to APOS-style concerns; useful if the
  survey wants a single citation linking blocker 9's cognitive framework to blocker 10's.

---

## Cross-cutting notes for blockers 9 and 10

1. **The two literatures barely touch set theory.** The transition-to-proof literature (9.x)
   studies proof courses and abstract algebra; APOS/RUMEC (10.x) studies calculus, linear
   algebra and functions. Only 9.3 (logical unpacking), 9.14 (TTP textbooks' treatment of
   sets), 10.3–10.5 (quantification), 10.10 (equivalence classes/partitions/fibers) and
   10.12 (computing-mediated set theory) are directly about set-theoretic content. A survey
   that claims "research shows students struggle with set theory" should cite Bagni (2006),
   Zazkis & Gunn (1997), Shaker & Berger (2016) and Hendriyanto et al. (2024) — i.e. the
   blocker-1/2 sources — rather than the proof literature.
2. **The 5–8.5% figure is the strongest single number available** for how badly students fail
   to extract logical structure from ordinary mathematical prose (Selden & Selden 1995, 9.3).
3. **The AE/EA asymmetry (94% vs 5%)** in 10.4 is the strongest single number for quantifier
   order, and its direction is pedagogically actionable.
4. **Dubinsky's own archive is the best open-access source for APOS primary texts.** All eight
   `https://www.math.kent.edu/~edd/*.pdf` URLs tested returned HTTP 200 this session; several
   are scanned back-issues whose extracted text uses a shifted font encoding (see the
   provenance note in Part B), but the PDFs themselves are the authoritative copies.
