package net.tikeo.boot.autoconfigure;

import lombok.Getter;
import lombok.Setter;
import org.springframework.boot.context.properties.ConfigurationProperties;

/**
 * Spring Boot properties for tikeo management/control-plane SDK clients.
 */
@Getter
@Setter
@ConfigurationProperties(prefix = "tikeo.management")
public class TikeoManagementProperties {
    /**
 * Enable management client auto-configuration.
 */
    private boolean enabled = false;
    /**
 * Tikeo HTTP management endpoint.
 */
    private String endpoint = "http://127.0.0.1:9999";
    /**
 * App-scoped API key used by management SDK clients.
 */
    private String apiKey = "";
    /**
 * Namespace scope for management operations.
 */
    private String namespace = "default";
    /**
 * App scope for management operations.
 */
    private String app = "default";
}
