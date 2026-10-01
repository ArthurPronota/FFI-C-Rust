// подключение стандартной библиотеки для математических функций
#include <math.h>

// определение структуры для представления точки в 2D пространстве
typedef struct {
    double x;
    double y;
} Point;

// функция для вычисления расстояния между двумя точками
double distance(Point p1, Point p2) {
    double dx = p2.x - p1.x;
    double dy = p2.y - p1.y;
    return sqrt(dx * dx + dy * dy);
}

