export const playUrl = (query, hash = "") => "http://127.0.0.1:8853/js/client/play.html?" + new URLSearchParams({ ci: 1, ...query }) + hash;
