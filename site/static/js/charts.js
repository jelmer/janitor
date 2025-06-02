/**
 * Chart.js Integration for Janitor
 * Provides data visualization charts for analytics and dashboards
 */

(function(window, document, $) {
    'use strict';

    // Ensure Janitor namespace exists
    window.Janitor = window.Janitor || {};

    /**
     * Charts Module
     */
    Janitor.Charts = (function() {
        
        // Chart instances registry
        var chartInstances = new Map();

        // Default Chart.js configuration
        var defaultConfig = {
            responsive: true,
            maintainAspectRatio: false,
            plugins: {
                legend: {
                    position: 'bottom',
                    labels: {
                        padding: 20,
                        usePointStyle: true
                    }
                },
                tooltip: {
                    backgroundColor: 'rgba(0, 0, 0, 0.8)',
                    titleColor: '#fff',
                    bodyColor: '#fff',
                    borderColor: '#666',
                    borderWidth: 1,
                    cornerRadius: 4,
                    displayColors: true
                }
            },
            animation: {
                duration: 750,
                easing: 'easeInOutQuart'
            }
        };

        // Color palette for consistent styling
        var colorPalette = [
            window.chartColors.blue,
            window.chartColors.red,
            window.chartColors.green,
            window.chartColors.orange,
            window.chartColors.purple,
            window.chartColors.yellow,
            window.chartColors.grey
        ];

        var lightColorPalette = [
            window.chartColors.lightBlue,
            window.chartColors.lightRed,
            window.chartColors.lightGreen,
            window.chartColors.lightOrange,
            window.chartColors.lightPurple,
            window.chartColors.lightYellow,
            window.chartColors.lightGrey
        ];

        /**
         * Generate colors for chart data
         */
        function generateColors(count, useLight) {
            var palette = useLight ? lightColorPalette : colorPalette;
            var colors = [];
            
            for (var i = 0; i < count; i++) {
                colors.push(palette[i % palette.length]);
            }
            
            return colors;
        }

        /**
         * Create a pie chart for result codes or failure stages
         */
        function createPieChart(canvasId, data, options) {
            var canvas = document.getElementById(canvasId);
            if (!canvas) {
                console.error('Canvas element not found:', canvasId);
                return null;
            }

            var ctx = canvas.getContext('2d');
            
            // Prepare data
            var labels = data.map(function(item) {
                return item.label || item.name || item.code;
            });
            
            var values = data.map(function(item) {
                return item.value || item.count;
            });

            var colors = generateColors(labels.length);
            var lightColors = generateColors(labels.length, true);

            var config = $.extend(true, {}, defaultConfig, {
                type: 'pie',
                data: {
                    labels: labels,
                    datasets: [{
                        data: values,
                        backgroundColor: colors,
                        borderColor: colors,
                        borderWidth: 2,
                        hoverBackgroundColor: lightColors,
                        hoverBorderColor: colors,
                        hoverBorderWidth: 3
                    }]
                },
                options: {
                    onClick: function(event, elements) {
                        if (elements.length > 0) {
                            var index = elements[0].index;
                            var item = data[index];
                            if (item.url) {
                                window.location.href = item.url;
                            } else if (options && options.onClick) {
                                options.onClick(item, index);
                            }
                        }
                    },
                    plugins: {
                        tooltip: {
                            callbacks: {
                                label: function(context) {
                                    var label = context.label || '';
                                    var value = context.raw;
                                    var total = context.dataset.data.reduce(function(a, b) { return a + b; }, 0);
                                    var percentage = ((value / total) * 100).toFixed(1);
                                    return label + ': ' + value.toLocaleString() + ' (' + percentage + '%)';
                                }
                            }
                        }
                    }
                }
            }, options);

            try {
                var chart = new Chart(ctx, config);
                chartInstances.set(canvasId, chart);
                return chart;
            } catch (error) {
                console.error('Failed to create pie chart:', error);
                return null;
            }
        }

        /**
         * Create a bar chart for comparisons
         */
        function createBarChart(canvasId, data, options) {
            var canvas = document.getElementById(canvasId);
            if (!canvas) {
                console.error('Canvas element not found:', canvasId);
                return null;
            }

            var ctx = canvas.getContext('2d');

            var labels = data.map(function(item) {
                return item.label || item.name;
            });
            
            var values = data.map(function(item) {
                return item.value || item.count;
            });

            var colors = generateColors(labels.length);

            var config = $.extend(true, {}, defaultConfig, {
                type: 'bar',
                data: {
                    labels: labels,
                    datasets: [{
                        data: values,
                        backgroundColor: colors,
                        borderColor: colors,
                        borderWidth: 1
                    }]
                },
                options: {
                    scales: {
                        y: {
                            beginAtZero: true,
                            ticks: {
                                callback: function(value) {
                                    return value.toLocaleString();
                                }
                            }
                        }
                    },
                    plugins: {
                        legend: {
                            display: false
                        },
                        tooltip: {
                            callbacks: {
                                label: function(context) {
                                    return context.label + ': ' + context.raw.toLocaleString();
                                }
                            }
                        }
                    },
                    onClick: function(event, elements) {
                        if (elements.length > 0) {
                            var index = elements[0].index;
                            var item = data[index];
                            if (item.url) {
                                window.location.href = item.url;
                            } else if (options && options.onClick) {
                                options.onClick(item, index);
                            }
                        }
                    }
                }
            }, options);

            try {
                var chart = new Chart(ctx, config);
                chartInstances.set(canvasId, chart);
                return chart;
            } catch (error) {
                console.error('Failed to create bar chart:', error);
                return null;
            }
        }

        /**
         * Create a line chart for time series data
         */
        function createLineChart(canvasId, data, options) {
            var canvas = document.getElementById(canvasId);
            if (!canvas) {
                console.error('Canvas element not found:', canvasId);
                return null;
            }

            var ctx = canvas.getContext('2d');

            var config = $.extend(true, {}, defaultConfig, {
                type: 'line',
                data: data,
                options: {
                    scales: {
                        x: {
                            type: 'time',
                            time: {
                                displayFormats: {
                                    day: 'MMM DD',
                                    week: 'MMM DD',
                                    month: 'MMM YYYY'
                                }
                            }
                        },
                        y: {
                            beginAtZero: true,
                            ticks: {
                                callback: function(value) {
                                    return value.toLocaleString();
                                }
                            }
                        }
                    },
                    interaction: {
                        intersect: false,
                        mode: 'index'
                    }
                }
            }, options);

            try {
                var chart = new Chart(ctx, config);
                chartInstances.set(canvasId, chart);
                return chart;
            } catch (error) {
                console.error('Failed to create line chart:', error);
                return null;
            }
        }

        /**
         * Create a doughnut chart (similar to pie but with center hole)
         */
        function createDoughnutChart(canvasId, data, options) {
            var canvas = document.getElementById(canvasId);
            if (!canvas) {
                console.error('Canvas element not found:', canvasId);
                return null;
            }

            var ctx = canvas.getContext('2d');
            
            var labels = data.map(function(item) {
                return item.label || item.name || item.code;
            });
            
            var values = data.map(function(item) {
                return item.value || item.count;
            });

            var colors = generateColors(labels.length);

            var config = $.extend(true, {}, defaultConfig, {
                type: 'doughnut',
                data: {
                    labels: labels,
                    datasets: [{
                        data: values,
                        backgroundColor: colors,
                        borderColor: '#fff',
                        borderWidth: 2
                    }]
                },
                options: {
                    cutout: '60%',
                    onClick: function(event, elements) {
                        if (elements.length > 0) {
                            var index = elements[0].index;
                            var item = data[index];
                            if (item.url) {
                                window.location.href = item.url;
                            } else if (options && options.onClick) {
                                options.onClick(item, index);
                            }
                        }
                    },
                    plugins: {
                        tooltip: {
                            callbacks: {
                                label: function(context) {
                                    var label = context.label || '';
                                    var value = context.raw;
                                    var total = context.dataset.data.reduce(function(a, b) { return a + b; }, 0);
                                    var percentage = ((value / total) * 100).toFixed(1);
                                    return label + ': ' + value.toLocaleString() + ' (' + percentage + '%)';
                                }
                            }
                        }
                    }
                }
            }, options);

            try {
                var chart = new Chart(ctx, config);
                chartInstances.set(canvasId, chart);
                return chart;
            } catch (error) {
                console.error('Failed to create doughnut chart:', error);
                return null;
            }
        }

        /**
         * Initialize charts from data attributes
         */
        function initChartsFromAttributes() {
            $('canvas[data-chart-type]').each(function() {
                var $canvas = $(this);
                var chartType = $canvas.data('chart-type');
                var dataUrl = $canvas.data('chart-data-url');
                var dataSource = $canvas.data('chart-data');
                var canvasId = this.id;

                if (!canvasId) {
                    console.warn('Canvas element missing ID for chart initialization');
                    return;
                }

                // Load data and create chart
                if (dataUrl) {
                    Janitor.Ajax.get(dataUrl)
                        .done(function(response) {
                            var data = response.data || response;
                            createChartByType(canvasId, chartType, data);
                        })
                        .fail(function() {
                            console.error('Failed to load chart data from:', dataUrl);
                        });
                } else if (dataSource) {
                    // Data provided inline
                    try {
                        var data = typeof dataSource === 'string' ? JSON.parse(dataSource) : dataSource;
                        createChartByType(canvasId, chartType, data);
                    } catch (error) {
                        console.error('Failed to parse chart data:', error);
                    }
                } else {
                    console.warn('No data source specified for chart:', canvasId);
                }
            });
        }

        /**
         * Create chart based on type
         */
        function createChartByType(canvasId, type, data, options) {
            switch (type) {
                case 'pie':
                    return createPieChart(canvasId, data, options);
                case 'bar':
                    return createBarChart(canvasId, data, options);
                case 'line':
                    return createLineChart(canvasId, data, options);
                case 'doughnut':
                    return createDoughnutChart(canvasId, data, options);
                default:
                    console.error('Unknown chart type:', type);
                    return null;
            }
        }

        /**
         * Update chart data
         */
        function updateChart(canvasId, newData) {
            var chart = chartInstances.get(canvasId);
            if (!chart) {
                console.warn('Chart not found:', canvasId);
                return;
            }

            try {
                if (Array.isArray(newData)) {
                    // Update dataset data
                    chart.data.labels = newData.map(function(item) {
                        return item.label || item.name || item.code;
                    });
                    chart.data.datasets[0].data = newData.map(function(item) {
                        return item.value || item.count;
                    });
                } else {
                    // Full data replacement
                    chart.data = newData;
                }
                
                chart.update();
            } catch (error) {
                console.error('Failed to update chart:', error);
            }
        }

        /**
         * Destroy a chart
         */
        function destroyChart(canvasId) {
            var chart = chartInstances.get(canvasId);
            if (chart) {
                chart.destroy();
                chartInstances.delete(canvasId);
            }
        }

        /**
         * Get chart instance
         */
        function getChart(canvasId) {
            return chartInstances.get(canvasId);
        }

        /**
         * Resize all charts
         */
        function resizeCharts() {
            chartInstances.forEach(function(chart) {
                chart.resize();
            });
        }

        // Initialize charts when DOM is ready
        $(document).ready(function() {
            initChartsFromAttributes();
        });

        // Handle window resize
        $(window).on('resize', function() {
            resizeCharts();
        });

        // Public API
        return {
            createPieChart: createPieChart,
            createBarChart: createBarChart,
            createLineChart: createLineChart,
            createDoughnutChart: createDoughnutChart,
            createChartByType: createChartByType,
            updateChart: updateChart,
            destroyChart: destroyChart,
            getChart: getChart,
            resizeCharts: resizeCharts,
            initChartsFromAttributes: initChartsFromAttributes,
            generateColors: generateColors,
            defaultConfig: defaultConfig
        };
    })();

})(window, document, jQuery);