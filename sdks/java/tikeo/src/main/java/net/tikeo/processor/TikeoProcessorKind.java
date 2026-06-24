package net.tikeo.processor;

/**
 * Explicit processor declaration kind used for structured worker registration.
 */
public enum TikeoProcessorKind {
    /**
     * Normal application processor selected by job processorName.
     */
    NORMAL,

    /**
     * Plugin processor selected by plugin processorType plus processorName.
     */
    PLUGIN;

    /**
     * @return true for the normal application processor kind.
     */
    public boolean isNormal() {
        return this == NORMAL;
    }
}
