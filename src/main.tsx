import "@fontsource-variable/noto-sans";
import "@mantine/charts/styles.css";
import { createTheme, MantineProvider, rem } from "@mantine/core";
import "@mantine/core/styles.css";
import ReactDOM from "react-dom/client";
import "./styles.css";

import { ModalsProvider } from "@mantine/modals";
import { App } from "./App";
import { AppErrorBoundary } from "./components/AppErrorBoundary";

const theme = createTheme({
  fontFamily: '"Noto Sans Variable", Inter, Avenir, Helvetica, Arial, sans-serif',
  fontSizes: {
    xs: rem(14),
    sm: "12",
    md: "14",
    lg: "16",
    xl: "18",
  },
});

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <MantineProvider theme={theme} defaultColorScheme="dark">
    <ModalsProvider>
      <AppErrorBoundary>
        <App />
      </AppErrorBoundary>
    </ModalsProvider>
  </MantineProvider>
);
