# freeRGB Website

Die deutschsprachige Produktwebsite für freeRGB, umgesetzt mit React, Vite, Tailwind CSS und shadcn/ui-Konventionen (Radix UI, CVA, wiederverwendbare UI-Komponenten).

```bash
npm install
npm run dev
```

Produktions-Build:

```bash
npm run build
```

Website **mit der echten Desktop-GUI als Browser-Demo**:

```bash
npm ci --prefix ../apps/desktop
npm run build:site
npm run preview
```

Der Startbutton öffnet die vollständige Demo in einem eigenen Tab. Architektur,
Simulation, Unterpfade und GitHub-Pages-Einrichtung: [WEB_DEMO.md](../docs/WEB_DEMO.md).
