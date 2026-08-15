<template>
  <p v-if="ready">{{ label }}</p>
</template>
<script>
export default {
  save(value, retry) {
    if (value) {
      return retry ? 1 : 0
    }
    return 0
  }
}
</script>
