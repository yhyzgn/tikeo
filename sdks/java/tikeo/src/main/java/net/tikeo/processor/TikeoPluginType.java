package net.tikeo.processor;

/**
 * Constrained plugin processor type values for {@link TikeoProcessor} declarations.
 */
public enum TikeoPluginType {
    /** No plugin type; required for normal processors. */
    NONE(""),
    /** SQL-oriented plugin processor. */
    SQL("sql"),
    /** HTTP/API plugin processor. */
    HTTP("http"),
    /** Notification plugin processor. */
    NOTIFICATION("notification"),
    /** Explicit extension point; pair with {@link TikeoProcessor#customPluginType()}. */
    CUSTOM("custom");

    private final String wireValue;

    TikeoPluginType(String wireValue) {
        this.wireValue = wireValue;
    }

    /**
     * Stable lowercase value sent to the tikeo server.
     *
     * @return plugin type wire value
     */
    public String wireValue() {
        return wireValue;
    }
}
