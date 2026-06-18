export type AccessTokenRequest = Partial<{
  forceRefresh: boolean;
}>;

export type FetchLike = (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>;

export type ApiClientOptions = {
  getAccessToken: (
    request?: AccessTokenRequest
  ) => Promise<string | undefined> | string | undefined;
} & Partial<{
  baseUrl: string;
  fetchImpl: FetchLike;
}>;

export type ApiRequestOptions = Partial<{
  method: string;
  body: unknown;
}>;

type ApiErrorPayload = Partial<{
  code: string;
  message: string;
}>;

export class ApiClientError extends Error {
  readonly status: number;
  readonly code: string;

  constructor(status: number, code: string, message: string) {
    super(message);
    this.status = status;
    this.code = code;
    this.name = "ApiClientError";
  }
}

export async function authenticatedRequest<T>({
  baseUrl,
  clientOptions,
  path,
  requestOptions = {},
}: {
  baseUrl: string;
  clientOptions: ApiClientOptions;
  path: string;
  requestOptions?: ApiRequestOptions;
}): Promise<T> {
  const response = await authenticatedResponse(baseUrl, clientOptions, path, requestOptions);
  if (response.status === 204) {
    return undefined as T;
  }
  return (await response.json()) as T;
}

export async function authenticatedBlobRequest({
  baseUrl,
  clientOptions,
  path,
}: {
  baseUrl: string;
  clientOptions: ApiClientOptions;
  path: string;
}): Promise<Blob> {
  const response = await authenticatedResponse(baseUrl, clientOptions, path, {});
  return response.blob();
}

export function defaultFetch(input: RequestInfo | URL, init?: RequestInit) {
  return globalThis.fetch(input, init);
}

async function authenticatedResponse(
  baseUrl: string,
  clientOptions: ApiClientOptions,
  path: string,
  requestOptions: ApiRequestOptions
): Promise<Response> {
  const fetchImpl = clientOptions.fetchImpl ?? defaultFetch;
  let response = await sendAuthenticated(fetchImpl, baseUrl, clientOptions, path, requestOptions);
  if (response.status === 401) {
    response = await retryAuthenticated(fetchImpl, baseUrl, clientOptions, path, requestOptions);
  }
  if (!response.ok) {
    throw await apiError(response);
  }
  return response;
}

async function sendAuthenticated(
  fetchImpl: FetchLike,
  baseUrl: string,
  clientOptions: ApiClientOptions,
  path: string,
  requestOptions: ApiRequestOptions
) {
  const token = await clientOptions.getAccessToken();
  if (!token) {
    throw new ApiClientError(401, "unauthorized", "missing access token");
  }
  return fetchImpl(`${baseUrl}${path}`, requestParts(token, requestOptions));
}

async function retryAuthenticated(
  fetchImpl: FetchLike,
  baseUrl: string,
  clientOptions: ApiClientOptions,
  path: string,
  requestOptions: ApiRequestOptions
) {
  const token = await clientOptions.getAccessToken({ forceRefresh: true });
  if (!token) {
    throw new ApiClientError(401, "unauthorized", "missing access token");
  }
  return fetchImpl(`${baseUrl}${path}`, requestParts(token, requestOptions));
}

function requestParts(token: string, options: ApiRequestOptions) {
  const headers = new Headers({ authorization: `Bearer ${token}` });
  let body: string | undefined;
  if (options.body !== undefined) {
    headers.set("content-type", "application/json");
    body = JSON.stringify(options.body);
  }
  return {
    method: options.method ?? "GET",
    headers,
    body,
  };
}

async function apiError(response: Response) {
  let payload: ApiErrorPayload = {};
  try {
    payload = (await response.json()) as ApiErrorPayload;
  } catch {
    payload = {};
  }
  return new ApiClientError(
    response.status,
    payload.code ?? "api_error",
    apiErrorMessage(response, payload)
  );
}

function apiErrorMessage(response: Response, payload: ApiErrorPayload) {
  return payload.message?.trim() || response.statusText.trim() || `HTTP ${response.status}`;
}
