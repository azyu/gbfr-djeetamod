import { CharacterType, EquipmentAnalysisResponse } from "@/types";
import { create } from "zustand";

const EMPTY_RESPONSE: EquipmentAnalysisResponse = {
  connected: false,
  characters: [],
};

export const characterTypeKey = (characterType: CharacterType): string =>
  typeof characterType === "string" ? characterType : `unknown-${characterType.Unknown}`;

type EquipmentAnalysisStore = {
  response: EquipmentAnalysisResponse;
  selectedCharacter: CharacterType | null;
  loadResponse: (response: EquipmentAnalysisResponse) => void;
  selectCharacter: (characterType: CharacterType) => void;
  reset: () => void;
};

export const useEquipmentAnalysisStore = create<EquipmentAnalysisStore>((set) => ({
  response: EMPTY_RESPONSE,
  selectedCharacter: null,
  loadResponse: (response) =>
    set((state) => {
      const selectedKey = state.selectedCharacter ? characterTypeKey(state.selectedCharacter) : null;
      const selectedCharacter =
        response.characters.find((character) => characterTypeKey(character.characterType) === selectedKey)
          ?.characterType ??
        response.characters[0]?.characterType ??
        null;
      return { response, selectedCharacter };
    }),
  selectCharacter: (selectedCharacter) => set({ selectedCharacter }),
  reset: () => set({ response: EMPTY_RESPONSE, selectedCharacter: null }),
}));
