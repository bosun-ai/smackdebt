<template>
  <ul v-if="ready && valid || forced">
    <li v-for="item in items">{{ item.ready ? item.name : 'wait' }}</li>
    <button @click="save">save</button>
  </ul>
</template>
<script setup lang="ts">
const helper = (value: boolean) => value ? 1 : 0
function save(value: boolean): number {
  if (value) return helper(value)
  return 0
}
</script>
<style>.ready { color: green; }</style>
