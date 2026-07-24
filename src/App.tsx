import { BrowserRouter, Route, Routes } from "react-router-dom";

import { Meter } from "./pages/Meter";

import { EquipmentAnalysis } from "./pages/EquipmentAnalysis";
import { ItemAnalysis } from "./pages/ItemAnalysis";
import Logs from "./pages/Logs";
import SettingsPage from "./pages/Settings";
import { IndexPage as LogIndexPage } from "./pages/logs/Index";
import { ViewPage as LogViewPage } from "./pages/logs/View";
import useCloseButtonAction from "./pages/useCloseButtonAction";

import "./App.css";

export const App = () => {
  useCloseButtonAction();

  return (
    <BrowserRouter>
      <Routes>
        <Route path="/" element={<Meter />} />
        <Route path="/logs" element={<Logs />}>
          <Route index element={<LogIndexPage />} />
          <Route path="equipment" element={<EquipmentAnalysis />} />
          <Route path="items" element={<ItemAnalysis />} />
          <Route path=":id" element={<LogViewPage />} />
          <Route path="settings" element={<SettingsPage />} />
        </Route>
      </Routes>
    </BrowserRouter>
  );
};
