# Archivum

**Archivum** is a deterministic, human-readable, and tool-agnostic backup archiving system designed for long-term reliability, portability, and correctness.

It separates **index (metadata)** from **data (tar archives)** so your backups remain usable even without Archivum.

---

## ✨ Key Principles

* **Standard formats only** – JSON + TAR
* **Human-readable index** – inspect backups without special tools
* **Deterministic restores** – exact paths, permissions, timestamps
* **Split archives** – size-limited TAR parts for cloud storage
* **No vendor lock-in** – works with any TAR-compatible tool

---

## 📦 Archive Layout

```
backup/
├── index.arc.json      # Metadata & file map (human readable)
├── data.part000.tar    # Data archive part
├── data.part001.tar
└── ...
```

* `index.arc.json` can be opened, searched, and versioned
* `data.partXXX.tar` can be extracted using any file manager

---

## 🚀 Usage

### Create archive

```bash
archivum create <source_dir> <output_dir> [max_gb]
```

Example:

```bash
archivum create photos backup 2
```

### List archive summary

```bash
archivum list backup/index.arc.json
```

### Restore archive

```bash
archivum restore backup/index.arc.json restored_dir
```

---

## 🔐 Encryption (Optional)

Archivum is designed to integrate seamlessly with **Ciph** for encryption.

Recommended flow:

```
archivum create data backup
ciph encrypt backup backup.ciph
```

Decrypt later:

```
ciph decrypt backup.ciph backup
archivum restore backup/index.arc.json restored
```

🔗 **Ciph repository:** [https://github.com/ankit-chaubey/ciph](https://github.com/ankit-chaubey/ciph)

---

## 📱 Mobile & Cloud Friendly

* TAR files open in **any file manager**
* Index is readable on phone
* Split size makes it ideal for cloud uploads

---

## 🛣️ Roadmap (Post May 2026)

Planned future enhancements (no breaking guarantees):

* Selective restore by search query
* Incremental snapshot support
* Compression layers (optional)
* Parallel tar writing
* Archive verification & integrity reports

---

## 👤 Author

**Made by Ankit Chaubey**
GitHub: [https://github.com/ankit-chaubey](https://github.com/ankit-chaubey)

---

## 📜 License

Licensed under the **Apache License 2.0**.

---

> Archivum is built for people who care about their data **years later**, not just today.
