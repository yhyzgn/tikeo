/**
 * App-scoped management API helpers.
 *
 * <p>Management clients use manually issued tikeo API keys and are not tied to browser login
 * sessions. They are intended for CI/CD, SDK demos, and service-owned automation that manages jobs in
 * an authorized namespace/app scope.
 *
 * <p><strong>Operational cautions:</strong> API keys are bearer secrets. Store them in a secret manager,
 * rotate them through the tikeo UI/API, and never write them to SDK diagnostics or task logs.
 */
package net.tikeo.management;
