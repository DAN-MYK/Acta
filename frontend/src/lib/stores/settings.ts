import { get, writable } from "svelte/store";
import {
  settingsBackupNow,
  settingsBackupOpenLatest,
  settingsConfigureIntegration,
  settingsLoad,
  settingsSaveCompany,
  settingsSavePreferences,
  settingsTeamInvite
} from "../api";
import type { SettingsCompanyDto, SettingsScreenDto, SettingsSection } from "../types";

interface SettingsState {
  screen: SettingsScreenDto | null;
  loading: boolean;
  error: string | null;
  message: string | null;
  section: SettingsSection;
}

const initialState: SettingsState = {
  screen: null,
  loading: false,
  error: null,
  message: null,
  section: "appearance"
};

function createSettingsStore() {
  const { subscribe, set, update } = writable<SettingsState>(initialState);

  return {
    subscribe,
    reset() {
      set(initialState);
    },
    async load() {
      update((state) => ({ ...state, loading: true, error: null }));

      try {
        const screen = await settingsLoad();
        update((state) => ({ ...state, screen, loading: false }));
        return screen;
      } catch (error) {
        update((state) => ({ ...state, loading: false, error: String(error) }));
        return null;
      }
    },
    setSection(section: SettingsSection) {
      update((state) => ({ ...state, section }));
    },
    updatePreference(field: "darkMode", value: boolean) {
      update((state) => ({
        ...state,
        screen: state.screen
          ? {
              ...state.screen,
              preferences: {
                ...state.screen.preferences,
                [field]: value
              }
            }
          : null
      }));
    },
    updateCompanyField(field: keyof SettingsCompanyDto, value: string | boolean) {
      update((state) => ({
        ...state,
        screen: state.screen
          ? {
              ...state.screen,
              company: {
                ...state.screen.company,
                [field]: value
              }
            }
          : null
      }));
    },
    async savePreferences() {
      const screen = get({ subscribe }).screen;
      if (!screen) {
        return null;
      }

      update((state) => ({ ...state, loading: true, error: null, message: null }));

      try {
        const result = await settingsSavePreferences(screen.preferences.darkMode);
        update((state) => ({
          ...state,
          screen: result.screen,
          loading: false,
          message: result.message
        }));
        return result;
      } catch (error) {
        update((state) => ({ ...state, loading: false, error: String(error) }));
        return null;
      }
    },
    async saveCompany() {
      const company = get({ subscribe }).screen?.company;
      if (!company) {
        return null;
      }

      update((state) => ({ ...state, loading: true, error: null, message: null }));

      try {
        const result = await settingsSaveCompany(company);
        update((state) => ({
          ...state,
          screen: result.screen,
          loading: false,
          message: result.message
        }));
        return result;
      } catch (error) {
        update((state) => ({ ...state, loading: false, error: String(error) }));
        return null;
      }
    },
    async configureIntegration(tag: string) {
      update((state) => ({ ...state, loading: true, error: null, message: null }));

      try {
        const result = await settingsConfigureIntegration(tag);
        update((state) => ({
          ...state,
          screen: result.screen,
          loading: false,
          message: result.message
        }));
        return result;
      } catch (error) {
        update((state) => ({ ...state, loading: false, error: String(error) }));
        return null;
      }
    },
    async inviteTeam() {
      update((state) => ({ ...state, loading: true, error: null, message: null }));

      try {
        const result = await settingsTeamInvite();
        update((state) => ({
          ...state,
          screen: result.screen,
          loading: false,
          message: result.message
        }));
        return result;
      } catch (error) {
        update((state) => ({ ...state, loading: false, error: String(error) }));
        return null;
      }
    },
    async backupNow() {
      update((state) => ({ ...state, loading: true, error: null, message: null }));

      try {
        const result = await settingsBackupNow();
        update((state) => ({
          ...state,
          screen: result.screen,
          loading: false,
          message: result.message
        }));
        return result;
      } catch (error) {
        update((state) => ({ ...state, loading: false, error: String(error) }));
        return null;
      }
    },
    async openLatestBackup() {
      update((state) => ({ ...state, loading: true, error: null, message: null }));

      try {
        const result = await settingsBackupOpenLatest();
        update((state) => ({
          ...state,
          loading: false,
          message: `${result.message}: ${result.path}`
        }));
        return result;
      } catch (error) {
        update((state) => ({ ...state, loading: false, error: String(error) }));
        return null;
      }
    }
  };
}

export const settingsStore = createSettingsStore();
