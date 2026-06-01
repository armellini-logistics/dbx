<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useI18n } from "vue-i18n";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Lock, Loader2, ShieldCheck } from "lucide-vue-next";
import AppLogo from "@/components/icons/AppLogo.vue";

const props = withDefaults(
  defineProps<{
    setupMode?: boolean;
  }>(),
  { setupMode: false },
);

const emit = defineEmits<{ authenticated: [] }>();
const { t } = useI18n();

const password = ref("");
const confirmPassword = ref("");
const error = ref("");
const loading = ref(false);

const googleAuthEnabled = ref(false);
const userEmail = ref<string | null>(null);

onMounted(async () => {
  try {
    const res = await fetch("/api/auth/check");
    const data = await res.json();
    googleAuthEnabled.value = !!data.google_auth_enabled;
    userEmail.value = data.user_email || null;
  } catch (e) {
    // Ignore
  }
});

function redirectToGoogle() {
  window.location.href = "/api/auth/google/login";
}

async function submit() {
  if (props.setupMode && password.value !== confirmPassword.value) {
    error.value = t("auth.passwordMismatch");
    return;
  }

  loading.value = true;
  error.value = "";
  try {
    const url = props.setupMode ? "/api/auth/setup" : "/api/auth/login";
    const res = await fetch(url, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ password: password.value }),
    });
    if (res.ok) {
      emit("authenticated");
    } else {
      const text = await res.text();
      error.value = text || t("auth.loginFailed");
    }
  } catch (e: any) {
    error.value = e?.message || t("auth.connectFailed");
  } finally {
    loading.value = false;
  }
}
</script>

<template>
  <div
    class="flex items-center justify-center h-screen bg-gradient-to-br from-background via-background to-blue-950/20"
  >
    <div class="w-[360px] space-y-8">
      <div class="flex flex-col items-center gap-4">
        <AppLogo class="w-20 h-20 rounded-2xl shadow-lg shadow-blue-500/20" />
        <div class="text-center">
          <h1 class="text-2xl font-bold tracking-tight">DBX</h1>
          <p class="text-sm text-muted-foreground mt-1">
            {{ setupMode ? t("auth.setupDescription") : t("auth.loginDescription") }}
          </p>
        </div>
      </div>

      <div v-if="googleAuthEnabled" class="space-y-6 bg-card/45 backdrop-blur-md p-6 rounded-2xl border border-border/80 shadow-xl">
        <p class="text-sm text-muted-foreground text-center leading-relaxed">
          Please sign in with your Google account to access your personalized database workspace.
        </p>
        <Button
          type="button"
          class="w-full h-12 bg-white text-black hover:bg-gray-100 dark:bg-zinc-950 dark:text-zinc-100 dark:hover:bg-zinc-900 border border-zinc-200 dark:border-zinc-800 font-semibold flex items-center justify-center gap-3 transition-all duration-300 transform hover:scale-[1.02] active:scale-[0.98] shadow-md hover:shadow-lg rounded-xl cursor-pointer"
          @click="redirectToGoogle"
        >
          <svg class="w-5 h-5" viewBox="0 0 24 24" fill="currentColor">
            <path d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z" fill="#4285F4"/>
            <path d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z" fill="#34A853"/>
            <path d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.06H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.94l2.85-2.22.81-.63z" fill="#FBBC05"/>
            <path d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.06l3.66 2.84c.87-2.6 3.3-4.52 6.16-4.52z" fill="#EA4335"/>
          </svg>
          Sign in with Google
        </Button>
      </div>

      <form v-else class="space-y-4" @submit.prevent="submit" autocomplete="off">
        <div v-if="setupMode" class="flex items-center justify-center gap-2 text-sm text-muted-foreground">
          <ShieldCheck class="w-4 h-4" />
          <span>{{ t("auth.setupTitle") }}</span>
        </div>
        <div class="relative">
          <Lock class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
          <Input
            v-model="password"
            type="password"
            :placeholder="setupMode ? t('auth.newPassword') : t('auth.enterPassword')"
            class="pl-10 h-11"
            autocomplete="off"
            autofocus
          />
        </div>
        <div v-if="setupMode" class="relative">
          <Lock class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
          <Input
            v-model="confirmPassword"
            type="password"
            :placeholder="t('auth.confirmPassword')"
            class="pl-10 h-11"
            autocomplete="off"
          />
        </div>
        <p v-if="error" class="text-sm text-destructive text-center">{{ error }}</p>
        <Button
          type="submit"
          class="w-full h-11 text-sm font-medium"
          :disabled="loading || !password || (setupMode && !confirmPassword)"
        >
          <Loader2 v-if="loading" class="w-4 h-4 animate-spin mr-2" />
          {{ loading ? t("auth.processing") : setupMode ? t("auth.setPassword") : t("auth.login") }}
        </Button>
      </form>

      <div class="text-center space-y-1 select-none">
        <p class="text-xs text-muted-foreground/50">Powered by DBX</p>
        <p class="text-[10px] text-muted-foreground/45 font-mono">Version 2.5.6 (Updated: 06/01/2026 04:15pm)</p>
      </div>
    </div>
  </div>
</template>
