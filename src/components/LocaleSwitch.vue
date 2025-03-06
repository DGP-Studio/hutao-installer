<template>
  <select v-model="selectedLocale" @change="changeLocale">
    <option v-for="lang in langList" :key="lang" :value="lang">
       &nbsp;{{ lang }}
    </option>
  </select>
</template>
<script lang="ts" setup>
import { useI18n } from 'vue-i18n';
import { ref } from 'vue';

const { locale } = useI18n();
const langList = ref<Array<string>>(['en', 'chs', 'cht', 'jp']);
const selectedLocale = ref<string>('en');

function changeLocale() {
  locale.value = selectedLocale.value;
}
</script>
<style lang="css" scoped>
select {
  height: 28px;
  width: 50px;
  border: unset;
  outline: none;
  appearance: none;
  -webkit-appearance: none;
  -moz-appearance: none;
  background: url('../../public/Locale.png') no-repeat;
  background-size: 28px 28px;
  color: transparent;
  cursor: pointer;

  option {
    color: #4f8bf5;
    background: #d9d9d9;
    display: flex;
    justify-content: center;
    align-items: center;
  }
}
</style>
