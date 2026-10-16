import axios from 'axios'

export const api = axios.create({
  baseURL: 'http://localhost:8080/v1',
  timeout: 10_000,
  headers: {
    Accept: 'application/json',
  },
  decompress: true,
  fetchOptions: {},
  withCredentials: true,
})
